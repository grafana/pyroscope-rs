#![no_std]

pub mod py313;
pub mod py314;

/// Convert a 3.14 `_Py_DebugOffsets` into the 3.13 layout (common denominator).
/// Fields only present in 3.14 are dropped; shared sub-structs are copied
/// field-by-field since the 3.14 versions may have extra trailing fields.
impl From<&py314::_Py_DebugOffsets> for py313::_Py_DebugOffsets {
    fn from(v: &py314::_Py_DebugOffsets) -> Self {
        Self {
            cookie: v.cookie,
            version: v.version,
            free_threaded: v.free_threaded,
            runtime_state: py313::_Py_DebugOffsets__runtime_state {
                size: v.runtime_state.size,
                finalizing: v.runtime_state.finalizing,
                interpreters_head: v.runtime_state.interpreters_head,
            },
            interpreter_state: py313::_Py_DebugOffsets__interpreter_state {
                size: v.interpreter_state.size,
                id: v.interpreter_state.id,
                next: v.interpreter_state.next,
                threads_head: v.interpreter_state.threads_head,
                // 3.14 has threads_main here; 3.13 does not — skip it
                gc: v.interpreter_state.gc,
                imports_modules: v.interpreter_state.imports_modules,
                sysdict: v.interpreter_state.sysdict,
                builtins: v.interpreter_state.builtins,
                ceval_gil: v.interpreter_state.ceval_gil,
                gil_runtime_state: v.interpreter_state.gil_runtime_state,
                gil_runtime_state_enabled: v.interpreter_state.gil_runtime_state_enabled,
                gil_runtime_state_locked: v.interpreter_state.gil_runtime_state_locked,
                gil_runtime_state_holder: v.interpreter_state.gil_runtime_state_holder,
            },
            thread_state: py313::_Py_DebugOffsets__thread_state {
                size: v.thread_state.size,
                prev: v.thread_state.prev,
                next: v.thread_state.next,
                interp: v.thread_state.interp,
                current_frame: v.thread_state.current_frame,
                thread_id: v.thread_state.thread_id,
                native_thread_id: v.thread_state.native_thread_id,
                datastack_chunk: v.thread_state.datastack_chunk,
                status: v.thread_state.status,
            },
            interpreter_frame: py313::_Py_DebugOffsets__interpreter_frame {
                size: v.interpreter_frame.size,
                previous: v.interpreter_frame.previous,
                executable: v.interpreter_frame.executable,
                instr_ptr: v.interpreter_frame.instr_ptr,
                localsplus: v.interpreter_frame.localsplus,
                owner: v.interpreter_frame.owner,
                // 3.14 has stackpointer here; 3.13 does not — drop it
            },
            code_object: py313::_Py_DebugOffsets__code_object {
                size: v.code_object.size,
                filename: v.code_object.filename,
                name: v.code_object.name,
                qualname: v.code_object.qualname,
                linetable: v.code_object.linetable,
                firstlineno: v.code_object.firstlineno,
                argcount: v.code_object.argcount,
                localsplusnames: v.code_object.localsplusnames,
                localspluskinds: v.code_object.localspluskinds,
                co_code_adaptive: v.code_object.co_code_adaptive,
            },
            pyobject: py313::_Py_DebugOffsets__pyobject {
                size: v.pyobject.size,
                ob_type: v.pyobject.ob_type,
            },
            type_object: py313::_Py_DebugOffsets__type_object {
                size: v.type_object.size,
                tp_name: v.type_object.tp_name,
                tp_repr: v.type_object.tp_repr,
                tp_flags: v.type_object.tp_flags,
            },
            tuple_object: py313::_Py_DebugOffsets__tuple_object {
                size: v.tuple_object.size,
                ob_item: v.tuple_object.ob_item,
                ob_size: v.tuple_object.ob_size,
            },
            list_object: py313::_Py_DebugOffsets__list_object {
                size: v.list_object.size,
                ob_item: v.list_object.ob_item,
                ob_size: v.list_object.ob_size,
            },
            dict_object: py313::_Py_DebugOffsets__dict_object {
                size: v.dict_object.size,
                ma_keys: v.dict_object.ma_keys,
                ma_values: v.dict_object.ma_values,
            },
            float_object: py313::_Py_DebugOffsets__float_object {
                size: v.float_object.size,
                ob_fval: v.float_object.ob_fval,
            },
            long_object: py313::_Py_DebugOffsets__long_object {
                size: v.long_object.size,
                lv_tag: v.long_object.lv_tag,
                ob_digit: v.long_object.ob_digit,
            },
            bytes_object: py313::_Py_DebugOffsets__bytes_object {
                size: v.bytes_object.size,
                ob_size: v.bytes_object.ob_size,
                ob_sval: v.bytes_object.ob_sval,
            },
            unicode_object: py313::_Py_DebugOffsets__unicode_object {
                size: v.unicode_object.size,
                state: v.unicode_object.state,
                length: v.unicode_object.length,
                asciiobject_size: v.unicode_object.asciiobject_size,
            },
            gc: py313::_Py_DebugOffsets__gc {
                size: v.gc.size,
                collecting: v.gc.collecting,
            },
            // 3.14 sub-structs not in 3.13: set_object, gen_object, debugger_support — dropped
        }
    }
}
