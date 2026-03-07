#![no_std]

use python_offsets_types::py313;

/// C-stack entry shim frame — skip during unwinding.
pub const FRAME_OWNED_BY_CSTACK: u64 = 3;

/// Maximum number of frames to collect in a single unwind.
pub const MAX_DEPTH: usize = 128;

/// A raw Python frame captured during unwinding.
///
/// Contains the `PyCodeObject*` address and the raw `instr_ptr` value.
/// The instruction offset from `co_code_adaptive` is computed later during
/// symbolication — storing the raw pointer avoids an extra `kindasafe::u64()`
/// read per frame in the signal handler.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawFrame {
    pub code_object: u64,
    pub instr_offset: u64,
}

/// Walk the Python interpreter frame chain starting from `tstate` and collect
/// raw `(code_object, instr_ptr)` tuples into `buf`.
///
/// Returns the number of frames written to `buf`.
///
/// All memory reads use `kindasafe::u64()` for signal-safety. On any read
/// failure the walk stops and returns whatever frames were collected so far.
///
/// Frames with `owner == FRAME_OWNED_BY_CSTACK` (C→Python entry shims) are
/// skipped. Cycle detection and a max depth of `min(buf.len(), MAX_DEPTH)`
/// prevent infinite loops from corrupted pointers.
pub fn unwind(tstate: u64, offsets: &py313::_Py_DebugOffsets, buf: &mut [RawFrame]) -> usize {
    let max = if buf.len() < MAX_DEPTH {
        buf.len()
    } else {
        MAX_DEPTH
    };

    let frame_ptr = match kindasafe::u64(tstate + offsets.thread_state.current_frame) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    if frame_ptr == 0 {
        return 0;
    }

    let mut frame_ptr = frame_ptr;
    let mut prev_frame: u64 = 0;
    let mut depth: usize = 0;

    while frame_ptr != 0 && frame_ptr != prev_frame && depth < max {
        let owner = match kindasafe::u64(frame_ptr + offsets.interpreter_frame.owner) {
            Ok(v) => v & 0xFF,
            Err(_) => break,
        };

        if owner == FRAME_OWNED_BY_CSTACK {
            prev_frame = frame_ptr;
            frame_ptr = match kindasafe::u64(frame_ptr + offsets.interpreter_frame.previous) {
                Ok(v) => v,
                Err(_) => break,
            };
            continue;
        }

        let code_obj = match kindasafe::u64(frame_ptr + offsets.interpreter_frame.executable) {
            Ok(v) => v,
            Err(_) => break,
        };
        if code_obj == 0 {
            break;
        }

        let instr_ptr = match kindasafe::u64(frame_ptr + offsets.interpreter_frame.instr_ptr) {
            Ok(v) => v,
            Err(_) => break,
        };

        buf[depth] = RawFrame {
            code_object: code_obj,
            instr_offset: instr_ptr,
        };

        notlibc::debug::writes("  [");
        notlibc::debug::write_hex(depth);
        notlibc::debug::writes("] code=0x");
        notlibc::debug::write_hex(code_obj as usize);
        notlibc::debug::writes(" instr=0x");
        notlibc::debug::write_hex(instr_ptr as usize);
        notlibc::debug::writes(" owner=");
        notlibc::debug::write_hex(owner as usize);
        notlibc::debug::puts("");

        depth += 1;

        prev_frame = frame_ptr;
        frame_ptr = match kindasafe::u64(frame_ptr + offsets.interpreter_frame.previous) {
            Ok(v) => v,
            Err(_) => break,
        };
    }

    notlibc::debug::writes("python_unwind: depth=");
    notlibc::debug::write_hex(depth);
    notlibc::debug::puts("");

    depth
}
