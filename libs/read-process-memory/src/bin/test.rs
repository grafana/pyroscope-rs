// This test program is used in the tests in src/lib.rs.
use std::env;
use std::io::{self, Read};

fn main() {
    let size = env::args()
        .nth(1)
        .and_then(|a| a.parse::<usize>().ok())
        .unwrap_or(32);
    let data = if size <= u8::max_value() as usize {
        (0..size as u8).collect::<Vec<u8>>()
    } else {
        (0..size)
            .map(|v| (v % (u8::max_value() as usize + 1)) as u8)
            .collect::<Vec<u8>>()
    };
    println!("{:p} {}", data.as_ptr(), data.len());
    // Wait to exit until stdin is closed.
    let mut buf = vec![];
    io::stdin().read_to_end(&mut buf).unwrap();
}
