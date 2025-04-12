

#[unsafe(no_mangle)]
pub extern "C" fn rust_ctor() {
    match kit::python::load() {
        Ok(_) => {
            println!("python loaded")
        }
        Err(err) => {
            println!("err {err:?}")
        }
    }
}

#[unsafe(link_section = ".init_array")]
pub static INITIALIZE: extern "C" fn() = rust_ctor;

