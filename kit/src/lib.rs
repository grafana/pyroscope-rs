pub mod tls;
mod python;

#[no_mangle]
pub extern "C" fn rust_ctor() {
    //todo this is executed when running tests LOL
    // make sure it does not happen, maybe extract this to a separate lib loader wihtout tests?
    match python::load() {
        Ok(_) => {
            println!("python loaded")
        }
        Err(err) => {
            println!("err {err:?}")
        }
    }
}

#[link_section = ".init_array"]
pub static INITIALIZE: extern "C" fn() = rust_ctor;

