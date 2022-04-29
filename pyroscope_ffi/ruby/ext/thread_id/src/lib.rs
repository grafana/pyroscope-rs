#[no_mangle]
pub extern "C" fn thread_id() -> u64 {
    unsafe { libc::pthread_self() as u64 }
}
