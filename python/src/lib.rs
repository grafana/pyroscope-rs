
#[unsafe(no_mangle)]
pub extern "C" fn rust_ctor() {
    if let Err(_) = kindasafe::init() {
        println!("failed to load kindasafe");
        return;
    }
    
    match kit::python::load() {
        Ok(_) => {
            println!("python loaded")
        }
        Err(err) => {
            println!("err {:?}", err)
        }
    }
}

// todo use pyo3
#[unsafe(link_section = ".init_array")]
pub static INITIALIZE: extern "C" fn() = rust_ctor;

