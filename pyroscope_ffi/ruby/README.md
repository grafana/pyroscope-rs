Pyroscope Ruby Integration --Beta--
=====================================

**note**: This is a beta release. It requires local compilation, might be
buggy and is frequently updated. For the initial implementation, find it [here](https://github.com/pyroscope-io/pyroscope-ruby). Please report any [issues](https://github.com/pyroscope-io/pyroscope-rs/issues).

## Installation

1. You need the Rust toolchain to compile the library locally. To install
   Rust:

```
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y 
export PATH=$PATH:/root/.cargo/bin
```

2. Building/Insalling from Rubygems 

```
gem install pyroscope_beta
```

3. Building/Installing from source

Change directory to `pyroscope_ffi/ruby` and run

```
gem build pyroscope.gemspec
gem install ./pyroscope.gemspec
```

## Configuration

Configuration is similar to the old package except for `application_name`:

```
require 'pyroscope_beta'

Pyroscope.configure do |config|
  config.application_name = "ruby.app"
  config.server_address = "http://localhost:4040"
  config.detect_subprocesses = true 
  config.tags = {
    :key => "value",
  }
end
```

## Adding tags

Tags passed to configure are global. To tag code locally, you can use:

```
Pyroscope.tag_wrapper({"profile": "profile-1"}) do
    // Tagged profile
end
```
