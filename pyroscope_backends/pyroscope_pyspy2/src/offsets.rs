use crate::version::Version;

#[derive(Debug)]
pub struct Offsets {
    pub PyVarObject_ob_size: isize,
    pub PyObject_ob_type: isize,
    pub PyTypeObject_tp_name: isize,
    pub PyThreadState_frame: isize,
    pub PyThreadState_cframe: isize,
    pub PyThreadState_current_frame: isize,
    pub PyCFrame_current_frame: isize,
    pub PyFrameObject_f_back: isize,
    pub PyFrameObject_f_code: isize,
    pub PyFrameObject_f_localsplus: isize,
    pub PyCodeObject_co_filename: isize,
    pub PyCodeObject_co_name: isize,
    pub PyCodeObject_co_varnames: isize,
    pub PyCodeObject_co_localsplusnames: isize,
    pub PyTupleObject_ob_item: isize,
    pub PyInterpreterFrame_f_code: isize,
    pub PyInterpreterFrame_f_executable: isize,
    pub PyInterpreterFrame_previous: isize,
    pub PyInterpreterFrame_localsplus: isize,
    pub PyInterpreterFrame_owner: isize,
    pub PyRuntimeState_gilstate: isize,
    pub PyRuntimeState_autoTSSkey: isize,
    pub Gilstate_runtime_state_autoTSSkey: isize,
    pub PyTssT_is_initialized: isize,
    pub PyTssT_key: isize,
    pub PyTssTSize: isize,
    pub PyASCIIObjectSize: isize,
    pub PyCompactUnicodeObjectSize: isize,
}




pub fn validate_python_offsets(ver: &Version, o: &Offsets) -> anyhow::Result<()> {
    fn require(o: isize, name: &str) -> anyhow::Result<()> {
        if o == -1 {
            return Err(anyhow::anyhow!("offset {} is required", name));
        }
        Ok(())
    }
    require(o.PyVarObject_ob_size, "PyVarObject_ob_size")?;
    require(o.PyObject_ob_type, "PyObject_ob_type")?;
    require(o.PyTypeObject_tp_name, "PyTypeObject_tp_name")?;
    require(o.PyThreadState_frame, "PyThreadState_frame")?;
    require(o.PyFrameObject_f_back, "PyFrameObject_f_back")?;
    require(o.PyFrameObject_f_code, "PyFrameObject_f_code")?;
    require(o.PyTssT_is_initialized, "PyTssT_is_initialized")?;
    require(o.PyCodeObject_co_name, "PyCodeObject_co_name")?;
    require(o.PyTssT_key, "PyTssT_key")?;
    require(o.PyTssTSize, "PyTssTSize")?;


    return Ok(());
}

//todo validate all offsets at build time or a test
pub fn get_python_offsets(ver: &Version) -> Offsets {
    return Offsets {
        PyVarObject_ob_size: 16,
        PyObject_ob_type: 8,
        PyTypeObject_tp_name: 24,
        PyThreadState_frame: 24,
        PyThreadState_cframe: -1,
        PyThreadState_current_frame: -1,
        PyCFrame_current_frame: -1,
        PyFrameObject_f_back: 24,
        PyFrameObject_f_code: 32,
        PyFrameObject_f_localsplus: 360,
        PyCodeObject_co_filename: 104,
        PyCodeObject_co_name: 112,
        PyCodeObject_co_varnames: 72,
        PyCodeObject_co_localsplusnames: -1,
        PyTupleObject_ob_item: 24,
        PyInterpreterFrame_f_code: -1,
        PyInterpreterFrame_f_executable: -1,
        PyInterpreterFrame_previous: -1,
        PyInterpreterFrame_localsplus: -1,
        PyInterpreterFrame_owner: -1,
        PyRuntimeState_gilstate: 1408,
        PyRuntimeState_autoTSSkey: -1,
        Gilstate_runtime_state_autoTSSkey: 32,
        PyTssT_is_initialized: 0,
        PyTssT_key: 8,
        PyTssTSize: 16,
        PyASCIIObjectSize: 48,
        PyCompactUnicodeObjectSize: 72,
    };
}
