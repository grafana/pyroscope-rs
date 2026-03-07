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
        assert_ne!(symbols.get_tstate_addr, 0);

        // ── Version detection ────────────────────────────────────────────
        let version = python_offsets::detect_version(symbols.py_version_addr)
            .map_err(|e| anyhow!("detect_version failed: {e:?}"))?;

        assert_eq!(version.major, 3);
        assert_eq!(version.minor, 14);

        // ── Debug offsets parsing ────────────────────────────────────────
        let version_hex = python_offsets::read_version_hex(symbols.py_version_addr)
            .map_err(|e| anyhow!("read_version_hex failed: {e:?}"))?;

        let offsets =
            python_offsets::read_debug_offsets(symbols.py_runtime_addr, &version, version_hex)
                .map_err(|e| anyhow!("read_debug_offsets failed: {e:?}"))?;

        // Returns py313 layout even for a 3.14 library (common denominator).
        // Verify key offsets are populated. Some offsets can legitimately
        // be 0 (e.g. executable if f_executable is the first field of
        // _PyInterpreterFrame, as it is in 3.14.3+).
        assert_ne!(offsets.runtime_state.interpreters_head, 0);
        assert_ne!(offsets.interpreter_state.threads_head, 0);
        assert_ne!(offsets.thread_state.native_thread_id, 0);
        assert_ne!(offsets.thread_state.next, 0);
        assert_ne!(offsets.interpreter_frame.previous, 0);
        assert_ne!(offsets.code_object.filename, 0);
        assert_ne!(offsets.code_object.qualname, 0);
        assert_ne!(offsets.unicode_object.asciiobject_size, 0);

        Ok(())
    }
}
