#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod linux {
    use anyhow::{Result, anyhow};
    use std::ffi::CString;

    const LIBPYTHON_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/libpython3.14.so.1.0");

    const ASYNCIO_SO_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/testdata/_asyncio.cpython-314-x86_64-linux-gnu.so"
    );

    /// dlopen a shared library with the given flags. Panics if the handle is null.
    fn dlopen_flags(path: &str, flags: libc::c_int) -> *mut libc::c_void {
        let cstr = CString::new(path).unwrap();
        let handle = unsafe { libc::dlopen(cstr.as_ptr(), flags) };
        assert!(
            !handle.is_null(),
            "dlopen({path}) failed: {}",
            unsafe { std::ffi::CStr::from_ptr(libc::dlerror()) }.to_string_lossy()
        );
        handle
    }

    /// dlopen with RTLD_LAZY | RTLD_NODELETE (private symbols).
    fn dlopen_or_panic(path: &str) -> *mut libc::c_void {
        dlopen_flags(path, libc::RTLD_LAZY | libc::RTLD_NODELETE)
    }

    /// dlopen with RTLD_LAZY | RTLD_NODELETE | RTLD_GLOBAL so that
    /// subsequently loaded libraries can resolve symbols from this one.
    fn dlopen_global(path: &str) -> *mut libc::c_void {
        dlopen_flags(
            path,
            libc::RTLD_LAZY | libc::RTLD_NODELETE | libc::RTLD_GLOBAL,
        )
    }

    #[test]
    fn end_to_end_python_offsets() -> Result<()> {
        kindasafe_init::init().map_err(|e| anyhow!("kindasafe_init::init failed: {e:?}"))?;

        // RTLD_NODELETE keeps the library resident so dlclose doesn't run the
        // FINI destructors. The handle is intentionally leaked (not dlclosed)
        // since the test process is short-lived.
        let _handle = dlopen_or_panic(LIBPYTHON_PATH);

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

        // Returns py314 layout directly.
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

    #[test]
    fn end_to_end_asyncio_offsets() -> Result<()> {
        kindasafe_init::init().map_err(|e| anyhow!("kindasafe_init::init failed: {e:?}"))?;

        // _asyncio.so depends on libpython symbols (PyTraceBack_Type, etc.),
        // so load libpython first with RTLD_GLOBAL to export its symbols.
        let _libpython = dlopen_global(LIBPYTHON_PATH);
        let _handle = dlopen_or_panic(ASYNCIO_SO_PATH);

        // ── Find _asyncio in /proc/self/maps ─────────────────────────────
        let binary = python_offsets::find_asyncio_in_maps()
            .map_err(|e| anyhow!("find_asyncio_in_maps failed: {e:?}"))?;

        assert!(
            binary.path.contains("_asyncio.cpython-314"),
            "unexpected path: {}",
            binary.path
        );
        assert_ne!(binary.base, 0);

        // ── Resolve _AsyncioDebug symbol ─────────────────────────────────
        let addr = python_offsets::resolve_asyncio_debug_symbol(&binary)
            .map_err(|e| anyhow!("resolve_asyncio_debug_symbol failed: {e:?}"))?;

        assert_ne!(addr, 0, "_AsyncioDebug address must be nonzero");

        // ── Read Py_AsyncioModuleDebugOffsets from live memory ───────────
        let offsets = python_offsets::read_asyncio_debug_offsets(addr)
            .map_err(|e| anyhow!("read_asyncio_debug_offsets failed: {e:?}"))?;

        // Validate task_object sub-struct: size must be nonzero (it's sizeof(TaskObj)),
        // and the field offsets must be nonzero (they are byte offsets into TaskObj).
        assert_ne!(
            offsets.asyncio_task_object.size, 0,
            "task_object.size must be nonzero"
        );
        assert_ne!(
            offsets.asyncio_task_object.task_name, 0,
            "task_object.task_name must be nonzero"
        );
        assert_ne!(
            offsets.asyncio_task_object.task_coro, 0,
            "task_object.task_coro must be nonzero"
        );
        assert_ne!(
            offsets.asyncio_task_object.task_node, 0,
            "task_object.task_node must be nonzero"
        );

        // Validate thread_state sub-struct: size must be nonzero (it's
        // sizeof(_PyThreadStateImpl)), and the offsets must be nonzero.
        assert_ne!(
            offsets.asyncio_thread_state.size, 0,
            "thread_state.size must be nonzero"
        );
        assert_ne!(
            offsets.asyncio_thread_state.asyncio_running_loop, 0,
            "thread_state.asyncio_running_loop must be nonzero"
        );
        assert_ne!(
            offsets.asyncio_thread_state.asyncio_running_task, 0,
            "thread_state.asyncio_running_task must be nonzero"
        );
        assert_ne!(
            offsets.asyncio_thread_state.asyncio_tasks_head, 0,
            "thread_state.asyncio_tasks_head must be nonzero"
        );

        // Validate interpreter_state sub-struct.
        assert_ne!(
            offsets.asyncio_interpreter_state.size, 0,
            "interpreter_state.size must be nonzero"
        );
        assert_ne!(
            offsets.asyncio_interpreter_state.asyncio_tasks_head, 0,
            "interpreter_state.asyncio_tasks_head must be nonzero"
        );

        // Sanity: task_object.size should be a reasonable struct size
        // (at least 100 bytes for TaskObj, at most 4096).
        assert!(
            offsets.asyncio_task_object.size >= 100 && offsets.asyncio_task_object.size <= 4096,
            "task_object.size {} looks unreasonable",
            offsets.asyncio_task_object.size
        );

        Ok(())
    }
}
