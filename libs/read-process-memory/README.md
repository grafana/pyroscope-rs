[![GitHub Actions Build status](https://github.com/rbspy/read-process-memory/actions/workflows/build.yml/badge.svg)](https://github.com/rbspy/read-process-memory/actions/workflows/build.yml) ![Cirrus CI Build status](https://api.cirrus-ci.com/github/luser/read-process-memory.svg) [![crates.io](https://img.shields.io/crates/v/read-process-memory.svg)](https://crates.io/crates/read-process-memory) [![](https://docs.rs/read-process-memory/badge.svg)](https://docs.rs/read-process-memory)

A crate to read memory from another process. Code originally taken from the [rbspy](https://github.com/rbspy/rbspy/) project. This crate has now returned home to the `rbspy` GitHub organization. :)

# Example

This example re-executes itself as a child process in order to have a separate process to use for demonstration purposes. If you need to read memory from a process that you are spawning, your usage should look very similar to this:

```rust
use std::convert::TryInto;
use std::env;
use std::io::{self, BufReader, BufRead, Read, Result};
use std::process::{Command, Stdio};

use read_process_memory::{
  Pid,
  ProcessHandle,
  CopyAddress,
  copy_address,
};

fn main() -> Result<()> {
    if env::args_os().len() > 1 {
      // We are the child.
      return in_child();
    }
    // Run this executable again so we have a child process to read.
    let mut child = Command::new(env::current_exe()?)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .arg("child")
        .spawn()?;

    // Get a ProcessHandle to work with.
    let handle: ProcessHandle = (&child).try_into().unwrap();

    // The child process will print the address to read from on stdout.
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut addr_string = String::new();
    stdout.read_line(&mut addr_string)?;
    let address = usize::from_str_radix(addr_string.trim(), 16).unwrap();

    // Try to read 10 bytes from that address
    let bytes = copy_address(address, 10, &handle)?;
    println!("Read: {:?}", bytes);

    // Tell the child to exit by closing its stdin.
    drop(child.stdin.take());
    // And wait for it to exit.
    child.wait()?;
    Ok(())
}

fn in_child() -> Result<()> {
    // Allocate a 10-byte Vec for the parent to read.
    let readable_bytes: Vec<u8> = vec![
        0xc0, 0x72, 0x80, 0x79, 0xeb, 0xf1, 0xbc, 0x87, 0x06, 0x14,
    ];
    // Print the address of the Vec to stdout so the parent can find it.
    println!("{:x}", readable_bytes.as_ptr() as usize);
    // Now wait to exit until the parent closes our stdin, to give
    // it time to read the memory.
    let mut buf = Vec::new();
    // We don't care if this succeeds.
    drop(io::stdin().read_to_end(&mut buf));
    Ok(())
}

```

# How it works

Here's a summary, with some C pseudocode, of how the `read-process-memory`
crate works under the hood on each of the platforms it supports. The three
inputs are:

* `PID`: the process ID to read from
* `LENGTH`: how much memory to read
* `ADDRESS`: the address to read from

## Linux:

Uses [process_vm_readv](https://man7.org/linux/man-pages/man2/process_vm_readv.2.html)

```c
void* TARGET = (void*) 0x123412341324;
struct iovec local;
local.iov_base = calloc(LENGTH, sizeof(char));
local.iov_len = LENGTH;
struct iovec remote;
remote[0].iov_base = TARGET;
remote[0].iov_len = LENGTH;
process_vm_readv(PID, local, 2, remote, 1, 0);
```

## Mac OS:

Uses [vm_read_overwrite](https://developer.apple.com/documentation/kernel/1585371-vm_read_overwrite)

```c
mach_port_name_t task;
task_for_pid(mach_task_self(), PID, &task)
vm_size_t read_len = LENGTH;
char result[LENGTH];
vm_read_overwrite(task, TARGET, LENGTH, &result, &read_len)
```

## FreeBSD:

Uses [ptrace](https://man.freebsd.org/cgi/man.cgi?query=ptrace). This one stops the process to read from it.

```c
// attach
int wait_status = 0;
attach_status = ptrace(PT_ATTACH, PID, null, 0);
waitpid(PID, &wait_status, 0);
WIFSTOPPED(wait_status)
char result[LENGTH];
desc = PtraceIoDesc {
  piod_op: PIOD_READ_D,
  piod_offs: TARGET;
  piod_addr: &result;
  piod_len: LENGTH,
};
// read data
ptrace(PT_IO, PID, &desc, 0);
// detach
ptrace(PT_DETACH, PID, null, 0);
```

## Windows:

Uses [ReadProcessMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory):

```c
char result[LENGTH];
ReadProcessMemory(PID, ADDRESS, &result, LENGTH, null);
```
