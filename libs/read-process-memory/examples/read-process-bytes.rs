extern crate libc;
extern crate read_process_memory;

use read_process_memory::*;
use std::convert::TryInto;
use std::env;

fn bytes_to_hex(bytes: &[u8]) -> String {
    let hex_bytes: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    hex_bytes.join("")
}

fn main() {
    let pid = env::args().nth(1).unwrap().parse::<usize>().unwrap() as Pid;
    let addr = usize::from_str_radix(&env::args().nth(2).unwrap(), 16).unwrap();
    let size = env::args().nth(3).unwrap().parse::<usize>().unwrap();
    let handle: ProcessHandle = pid.try_into().unwrap();
    copy_address(addr, size, &handle)
        .map_err(|e| {
            println!("Error: {:?}", e);
            e
        })
        .map(|bytes| {
            println!(
                "{} bytes at address {:x}:
{}
",
                size,
                addr,
                bytes_to_hex(&bytes)
            )
        })
        .unwrap();
}
