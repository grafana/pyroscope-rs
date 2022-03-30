Pyroscope CLI (pyroscope-cli)
-----------------------------

`pyroscope-cli` is a general purpose profiler. It currently supports profiling ruby and python applications. The aggregated data from profiling is then sent to a [Pyroscope Server](https://pyroscope.io/docs/installing-pyroscope-overview/). Under the hood, it uses the [Pyroscope Rust library](https://github.com/pyroscope-io/pyroscope-rs) and its [backends](https://github.com/pyroscope-io/pyroscope-rs/tree/main/pyroscope_backends).

This is a Work-in-Progress implementation. Some features (like adhoc/pull mode) are still not available and the profiling spies are limited to Ruby/Python. For the original implementation, you should check the [Pyroscope Go agent](https://pyroscope.io/docs/agent-overview/j). 

### CHANGELOG

Please see the [CHANGELOG](CHANGELOG.md) for a release history.

### Table of Contents

- [Installation](#installation)
- [How to use](#how-to-use)
- 1. [Basic Usage](#1-basic-usage)
- 2. [Connect to process](#2-connect-to-process)
- 3. [Execute commands](#3-execute-commands)
- 4. [Options](#4-options)
- 5. [Configuration](#5-configuration)
- 5. 1. [Configuration file](#1-configuration-file)
- 5. 2. [Environment variables](#2-environment-variables)
- 6. [Logging](#6-logging)
- [Supported Profilers](#supported-profilers)
- 1. [rbspy](#1-rbspy)
- 2. [pyspy](#2-pyspy)
- [Frequently Asked Questions](#frequently-asked-questions)
- [Shell Completions](#shell-completions)
- [Building from source](#building-from-source)
- [License](#license)

### Installation

Currently, the best method to locally install `pyrsocope-cli` is to use the rustc compiler with Cargo.

```
$ cargo install pyrsocope-cli
```

Binaries are also available in the Release page. The targeted platforms are `x86_64`/`ARM` and `linux`/`macos`.

### How to use

#### 1. Basic Usage

There are two options to profile programs, regardless of the profiler: Either by connecting to the process [PID](https://en.wikipedia.org/wiki/Process_identifier), or by passing a command where the agent will handle both its execution and profiling.

```
$ pyroscope-cli connect --pid=$pid --spy-name=rbspy
```
```
$ pyroscope-cli exec --spy-name=rbspy ruby ./program.rb
```

#### 2. Connect to Process

To connect to a process and attach a profiler, you'll need both a PID and the required system privileges. The last one will vary depending on your Operating System and its configuration.

To get the PID of a program, you can use `ps` and `grep`:

```
$ ps -aux | grep ruby
```

You also need to specify the profiler, the possible values are rbspy (for Ruby) and pyspy (for Python). The `pid` and `spy-name` are the two required arguments to profile a process.

```
$ pyroscope-cli connect --pid=1222 --spy-name=rbspy
```


#### 3. Execute Commands

`pyroscope-cli` can execute a command and profile the spawned process. The command is spawned as a child of the agent process. Once the agent process exits, the executed command its child processes exit too.

```
$ pyroscope-cli exec --spy-name=rbspy ruby ./program.rb
```

You can also pass arguments to the executed command by appending `--`

```
$ pyroscope-cli exec --spy-name=rbspy ruby ./program.rb -- --ruby-arg=value
```

#### 4. Options

Both the pyroscope-cli agent, and its backend profilers can accept configuration. Some options are accepted by all profilers, while other can only apply to a certain profiler or a multiple of them. The CLI `--help` menu should give a detailed list of all options that the program can accept.

##### 4.1 Options accepted by all profilers and commands

- **application-name**: application name used when uploading profiling data. Default is a randomly generated name.
- **log-level**: log level for the application. Default is `info`. For more information, check [logging](#6-logging).
- **sample-rate**: sample rate for the profiler in Hz. 100 means reading 100 times per second. Default is `100`
- **server-address**: Pyroscope server address. Default is `http://localhost:4040`.
- **tag**: tag in key=value form. May be specified multiple times. Default is empty.

##### 4.2 Options accepted by `exec` command

- **user-name**: start process under specified user name.
- **group-name**: start process under specified group name.

#### 5. Configuration

There are 3 ways to configure Pyroscope Agent. Configuration precedence is evaluated in the following order: environment variables > configuration files > command-line arguments.

  ##### 1. Configuration file
  
  Configuration files are stored in [TOML format](https://en.wikipedia.org/wiki/TOML). You can specify configuration file location with `-config <path>`. This is supported for both `exec` and `connect` commands.
  
  ```
  pyroscope-cli -c -config /tmp/custom-config.toml <COMMAND>
  ```

  ##### 2. Environment variables
  
  Environment variables must have `PYROSCOPE_` prefix and be in UPPER_SNAKE_CASE format, for example:
  
  ```
  PYROSCOPE_APPLICATION_NAME=:my-ruby-app pyroscope-cli connect --pid=100 --spy-name=rbspy
  ```

#### 6. Logging

Logs are output to the terminal. There are 6 levels of logging. Log levels are not displayed seperately but rather takes precedence. For example, if you specify the `info` log level, you'll get output for `info`, `warn`, `error` and `critical` logs.

- **trace**: very low priority, often extremely verbose, information.
- **debug**: lower priority information.
- **info**: useful information.
- **warn**: hazardous situations.
- **error**: very serious errors.
- **critical**: errors that result in program panic.

### Supported Profilers

  #### 1. rbspy
  
  The `rbspy` profiler can be used to profile Ruby application. It uses the rbspy backend, which itself is a wrapper around the rbspy profiler.
  
  ##### Options accepted by rbspy
  
  - **detect-subprocesses**: keep track of and profile subprocesses of the main process.
  - **blocking**: enable blocking mode

  #### 2. pyspy
  
  The `pyspy` profiler can be used to profile Ruby application. It uses the pyspy backend, which itself is a wrapper around the py-spy profiler.
  
  ##### Options accepted by pyspy
  
  - **detect-subprocesses**: keep track of and profile subprocesses of the main process.
  - **blocking**: enable blocking mode.
  - **pyspy-idle**: include idle threads.
  - **pyspy-gil**: enable GIL mode.
  - **pyspy-native**: enable native extensions profiling.

### Frequently Asked Questions

Please see the [FAQ](FAQ.md) page for Frequently Asked Questions.

### Shell Completions

`pyroscope-cli` supports shell auto-completion for `bash`, `zsh`, `fish`, and `powershell`. You can generate the auto-completion file using the `completion` command.

For example, to generate auto-complete for `fish`:

```fish
$ pyroscope-cli completion fish > pyroscope-cli.fish
```

### Building from source

You can build `pyroscope-cli` from source if you have a Rust toolchain installed. You will need Rust 1.59 or newer.

1. Install Rust toolchain with rustup:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

For other systems, please check the instructions at https://rustup.rs/.

2. Add ~/.cargo/bin to your PATH:

```
. "$HOME/.cargo/env"
```

3. Build `pyroscope-cli`

```
git clone https://github.com/pyroscope-io/pyroscope-rs
cd pyroscope-rs/pyroscope_cli
cargo build --release
./target/release/pyroscope-cli --help
```

### License

Pyroscope is distributed under the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.
