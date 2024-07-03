/// Read bytes from the current process.
use read_process_memory::*;
use std::convert::TryInto;

fn main() {
    let data = vec![17u8, 23u8, 45u8, 0u8];
    let pid = unsafe { libc::getpid() } as Pid;
    let addr = data.as_ptr() as usize;
    let handle: ProcessHandle = pid.try_into().unwrap();
    copy_address(addr, 4, &handle)
        .map_err(|e| {
            println!("Error: {:?}", e);
            e
        })
        .map(|bytes| {
            assert_eq!(bytes, vec![17u8, 23u8, 45u8, 0u8]);
            println!("Success!")
        })
        .unwrap();
}
