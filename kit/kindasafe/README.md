# kindasafe

Signal-safe memory reading for x86_64 and aarch64 on Linux.

Uses naked assembly functions to read memory, with crash recovery via SIGSEGV/SIGBUS signal handlers. When a read faults, the signal handler adjusts the program counter to skip past the faulting instruction and reports the error instead of crashing.

This is a `no_std` crate providing the core read primitives. Use [`kindasafe_init`](https://crates.io/crates/kindasafe_init) to install the required signal handlers.

## Usage

```rust
// First, initialize the signal handlers (requires kindasafe_init)
kindasafe_init::init().expect("failed to init");

// Read a u64 from a potentially-invalid address
let value = kindasafe::u64(some_address);
match value {
    Ok(v) => println!("read: {v:#x}"),
    Err(e) => println!("fault: signal {}", e.signal),
}
```

## Supported architectures

- x86_64
- aarch64

## License

Apache-2.0
