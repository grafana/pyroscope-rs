# kindasafe_init

Signal handler initialization for the [kindasafe](https://crates.io/crates/kindasafe) signal-safe memory reading library.

Installs SIGSEGV and SIGBUS signal handlers that enable `kindasafe` to recover from memory access faults instead of crashing. Preserves any previously installed signal handlers as fallbacks.

## Usage

```rust
kindasafe_init::init().expect("failed to initialize kindasafe");

// Now kindasafe reads will recover from faults
let result = kindasafe::u64(some_address);
```

## License

Apache-2.0
