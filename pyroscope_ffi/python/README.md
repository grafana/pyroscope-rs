Pyroscope Python Integration --Beta--
=====================================

**note**: This is a beta release. It requires local compilation, might be
buggy and is frequently updated. For the initial implementation, find it [here](https://github.com/pyroscope-io/pyroscope-python). Please report any [issues](https://github.com/pyroscope-io/pyroscope-rs/issues).

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
import pyroscope_beta

pyroscope_beta.configure(
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
with pyroscope_beta.tag_wrapper({ "profile": "profile-1" }):
    // Tagged profile
```
