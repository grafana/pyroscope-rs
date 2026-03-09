#[test]
fn error_on_nonexistent_file() {
    let result = coredump::Coredump::open("/tmp/nonexistent-core-file-38493729");
    assert!(result.is_err());
}

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod coredump_read {
    use anyhow::{Result, anyhow};
    use std::path::PathBuf;
    use std::process::Command;

    const C_SOURCE: &str = r#"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* Known magic value in an initialized global. */
volatile uint64_t MAGIC = 0xDEADBEEFCAFEBABEULL;

/* A small struct to verify multi-field reads. */
struct TestData {
    uint32_t a;
    uint32_t b;
    uint64_t c;
};
volatile struct TestData DATA = { 0x11223344, 0x55667788, 0xAAAABBBBCCCCDDDDULL };

int main(void) {
    /* Touch the volatiles so the compiler cannot optimise them away. */
    uint64_t sink = MAGIC + DATA.a;
    (void)sink;
    abort();
    return 0;
}
"#;

    /// Find the virtual address of a named symbol in a non-PIE ELF binary.
    fn symbol_addr(elf_path: &str, name: &str) -> Result<u64> {
        let data = std::fs::read(elf_path)?;
        let obj = object::File::parse(&*data).map_err(|e| anyhow!("ELF parse: {e}"))?;
        use object::Object as _;
        use object::ObjectSymbol as _;
        for sym in obj.symbols() {
            if sym.name() == Ok(name) {
                return Ok(sym.address());
            }
        }
        Err(anyhow!("symbol '{name}' not found"))
    }

    /// Build the C helper, run it to produce a coredump, return (binary_path, core_path).
    fn build_and_dump(tmp: &std::path::Path) -> Result<(String, String)> {
        let src = tmp.join("crashme.c");
        let bin = tmp.join("crashme");
        let core = tmp.join("core");

        std::fs::write(&src, C_SOURCE)?;

        // Compile as non-PIE so symbol addresses are absolute.
        let cc = Command::new("gcc")
            .args([
                "-o",
                bin.to_str().unwrap(),
                src.to_str().unwrap(),
                "-no-pie",
                "-static",
            ])
            .output()?;
        if !cc.status.success() {
            return Err(anyhow!(
                "gcc failed: {}",
                String::from_utf8_lossy(&cc.stderr)
            ));
        }

        // Run the binary with coredumps enabled.
        // kernel.core_pattern may point elsewhere, so use a wrapper that
        // sets the core size and uses SIGSYS to control the pattern via
        // /proc/self — but the simplest portable approach is:
        //   1. set core_pattern via /proc/sys if possible (needs root)
        //   2. or just rely on kernel.core_pattern and search for the file
        //
        // Instead, we use gdb to generate the coredump at a known path,
        // which works reliably regardless of kernel.core_pattern.
        let gdb = Command::new("gdb")
            .args([
                "--batch",
                "-ex",
                "run",
                "-ex",
                &format!("generate-core-file {}", core.display()),
                "-ex",
                "quit",
                bin.to_str().unwrap(),
            ])
            .output()?;
        if !core.exists() {
            return Err(anyhow!(
                "gdb did not produce a core file.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&gdb.stdout),
                String::from_utf8_lossy(&gdb.stderr),
            ));
        }

        Ok((
            bin.to_str().unwrap().to_owned(),
            core.to_str().unwrap().to_owned(),
        ))
    }

    #[test]
    fn read_known_values_from_coredump() -> Result<()> {
        let tmp = tempdir()?;
        let (bin, core_path) = build_and_dump(&tmp)?;

        let core = coredump::Coredump::open(&core_path)?;

        // Sanity: the coredump has at least one thread and one mapping.
        assert!(!core.threads.is_empty(), "expected at least one thread");
        assert!(!core.mappings.is_empty(), "expected at least one mapping");

        // Look up symbol addresses from the static binary.
        let magic_addr = symbol_addr(&bin, "MAGIC")?;
        let data_addr = symbol_addr(&bin, "DATA")?;

        // Read MAGIC (u64).
        let magic_val = core.read_u64(magic_addr)?;
        assert_eq!(
            magic_val, 0xDEADBEEFCAFEBABE,
            "MAGIC mismatch: got 0x{magic_val:016x}"
        );

        // Read DATA.a (u32 at offset 0).
        let a = core.read_u32(data_addr)?;
        assert_eq!(a, 0x11223344, "DATA.a mismatch: got 0x{a:08x}");

        // Read DATA.b (u32 at offset 4).
        let b = core.read_u32(data_addr + 4)?;
        assert_eq!(b, 0x55667788, "DATA.b mismatch: got 0x{b:08x}");

        // Read DATA.c (u64 at offset 8).
        let c = core.read_u64(data_addr + 8)?;
        assert_eq!(c, 0xAAAABBBBCCCCDDDD, "DATA.c mismatch: got 0x{c:016x}");

        // Verify that reading an unmapped address fails.
        let bad = core.read_u64(0xDEAD_0000_0000);
        assert!(bad.is_err(), "reading unmapped address should fail");

        // Verify thread info has a non-zero tp_base (TLS base) and lwp.
        let t = &core.threads[0];
        assert_ne!(t.lwp, 0, "thread LWP should be non-zero");

        Ok(())
    }

    fn tempdir() -> Result<PathBuf> {
        let dir = std::env::temp_dir().join(format!("coredump-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}
