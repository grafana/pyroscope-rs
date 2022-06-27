Pyroscope Python Integration
============================

**note**: This is an early release. It might require local compilation, might be
buggy and will be frequently updated. For the initial implementation, revert
to version 2.x.

## Installation

1. You need the Rust toolchain to compile the library locally. To install
   Rust:

```
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y 
export PATH=$PATH:/root/.cargo/bin
```

2. libunwind8-dev is also required. For Ubuntu:

```
apt-get install -y libunwind8-dev 
```

3. Building/Insalling from PyPi package

```
pip install pyroscope_beta
```

4. Building/Installing from source

Change directory to `pyroscope_ffi/python` and run

```
make install
```

## Configuration

Configuration is similar to the old package except for `application_name`:

```
import pyroscope

pyroscope.configure(
  application_name       = "python.app",
  server_address         = "http://localhost:4040",

  tags = {
    "key": "value",
  }
)
```

## Adding tags

Tags passed to configure are global. To tag code locally, you can use:

```
with pyroscope.tag_wrapper({ "profile": "profile-1" }):
    // Tagged profile
```
