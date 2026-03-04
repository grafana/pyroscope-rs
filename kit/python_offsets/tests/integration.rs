#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod linux {
    use anyhow::{Result, anyhow};
    use std::ffi::CString;

    const LIBPYTHON_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/libpython3.14.so.1.0");

    #[test]
    fn end_to_end_python_offsets() -> Result<()> {
        kindasafe_init::init().map_err(|e| anyhow!("kindasafe_init::init failed: {e:?}"))?;

        // RTLD_NODELETE keeps the library resident so dlclose doesn't run the
        // FINI destructors. The handle is intentionally leaked (not dlclosed)
        // since the test process is short-lived.
        let path_cstr =
            CString::new(LIBPYTHON_PATH).map_err(|e| anyhow!("CString::new failed: {e}"))?;
        let handle =
            unsafe { libc::dlopen(path_cstr.as_ptr(), libc::RTLD_LAZY | libc::RTLD_NODELETE) };
        assert!(!handle.is_null());

        let binary = python_offsets::find_python_in_maps()
            .map_err(|e| anyhow!("find_python_in_maps failed: {e:?}"))?;

        assert!(binary.path.contains("libpython3"));

        let symbols = python_offsets::resolve_elf_symbols(&binary)
            .map_err(|e| anyhow!("resolve_elf_symbols failed: {e:?}"))?;

        assert_ne!(symbols.py_runtime_addr, 0);
        assert_ne!(symbols.py_version_addr, 0);

        // Py_Version is a uint32_t: (major<<24)|(minor<<16)|(micro<<8)|release_level
        // kindasafe::u64 reads 8 bytes; mask to 32 bits to isolate the version word.
        let raw = kindasafe::u64(symbols.py_version_addr)
            .map_err(|e| anyhow!("kindasafe::u64(py_version_addr) failed: {e:?}"))?;

        let version_u32 = (raw & 0xFFFF_FFFF) as u32;
        let major = (version_u32 >> 24) & 0xFF;
        let minor = (version_u32 >> 16) & 0xFF;

        assert_eq!(major, 3, "Py_Version major must be 3 (raw=0x{raw:016x})");
        assert_eq!(minor, 14, "Py_Version minor must be 14 (raw=0x{raw:016x})");

        Ok(())
    }
}
