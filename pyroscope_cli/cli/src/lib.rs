use clap::{AppSettings, ArgEnum, ArgMatches, IntoApp, Parser, Subcommand};
use clap_complete::{
    generate,
    shells::{Bash, Fish, PowerShell, Zsh},
};
use std::path::PathBuf;

use core::commands;
use utils::app_config::AppConfig;
use utils::error::Result;
use utils::types::{LogLevel, OutputFormat, Spy};

#[derive(Parser, Debug)]
#[clap(
    name = "pyroscope-cli",
    author,
    about,
    long_about = "Pyroscope CLI",
    version
)]
#[clap(setting = AppSettings::SubcommandRequired)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
pub struct Cli {
    /// Set a custom config file
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Subcommands
    #[clap(subcommand)]
    command: Commands,
}

/// Pyroscope CLI Commands
#[derive(Subcommand, Debug)]
enum Commands {
    #[clap(
        name = "adhoc",
        about = "Profile a process and save the results to be used in adhoc mode",
        long_about = "
adhoc command is a complete toolset to profile a process and save the profiling
results.
These results are generated in two different ways:
- Pyroscope stores the profiling data in its native format and data directory
by default. These profiles are then available for analysis using pyroscope UI
(the 'Adhoc Profiling' section), which requires a running pyroscope server.
This output can be disabled with '--no-json-output'.
- Pyroscope also generates profiling data in an external format.
Depending on the number of generated profiles, pyroscope will generate either
a file or a directory, in the directory where pyroscope is run from.
The currently supported formats are standalone HTML (which can then be
shared or directly open in a browser to analyze), pprof or collapsed
(these last two can then be shared and used with either pyroscope UI or other
tooling). The flag '--output-format' is used to specify this format.
There are multiple ways to gather the profiling data, and not all of them are
available for all the languages.
Which way is better depends on several factors: what the language supports,
how the profiled process is launched, and how the profiled process provides
the profiled data.
The different supported ways are:
- exec. In this case, pyroscope creates a different process for the profiled
program and uses a spy to directly gather profiling data. It's a useful way
to profile a whole execution of some program that has no other pyroscope
integration or way of exposing profiling data.
It's the default mode for languages with a supported spy when either the spy
name is specified (through '--spy-name' flag) or when the spyname is
autodetected.
- connect. Similar to exec, pyroscope uses a spy to gather profiling data,
but instead of creating a new profiled process, it spies an already running
process, indicated through '--pid' flag.
- push. In this case, pyroscope creates a different process for the profiled
program and launches an HTTP server with an ingestion endpoint. It's useful
to profile programs already integrated with Pyroscope using its HTTP API.
Push mode is used by default when no spy is detected and no '--url' flag is
provided. It can also be override the default exec mode with the '--push' flag.
- pull. In this case, pyroscope periodically connects to the URL specified
thorugh '--url' where it tries to retrieve profiling data in any of the
supported formats. In this case arguments are optional, and if provided,
they are used to launch a new process before polling the URL.
"
    )]
    Adhoc {
        #[clap(
            name = "application_name",
            long = "application-name",
            value_name = "APPLICATION_NAME",
            help = "application name used when uploading profiling data"
        )]
        application_name: Option<String>,
        #[clap(
            name = "data_path",
            long = "data-path",
            value_name = "DATA_PATH",
            help = "directory where pyroscope stores adhoc profiles"
        )]
        data_path: String,
        #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process"
        )]
        detect_subprocesses: bool,
        #[clap(
            name = "duration",
            long = "duration",
            value_name = "DURATION",
            help = "duration of the profiling session, which is the whole execution of the profiled process by default",
            default_value = "0s"
        )]
        duration: String,
        #[clap(
            name = "log_level",
            short,
            long = "log-level",
            value_name = "LOG_LEVEL",
            help = "",
            default_value = "info"
        )]
        log_level: LogLevel,
        #[clap(
            name = "no_logging",
            long = "no-logging",
            value_name = "NO_LOGGING",
            help = "disable logging from pyroscope"
        )]
        no_logging: bool,
        #[clap(
            name = "max_nodes_render",
            long = "max-nodes-render",
            value_name = "MAX_NODES_RENDER",
            help = "max number of nodes used to display data on the frontend",
            parse(try_from_str),
            default_value = "8192"
        )]
        max_nodes_render: u32,
        #[clap(
            name = "max_nodes_serialization",
            long = "max-nodes-serialization",
            value_name = "MAX_NODES_SERIALIZATION",
            help = "max number of nodes used when saving profiles to disk",
            parse(try_from_str),
            default_value = "2048"
        )]
        max_nodes_serialization: u32,
        #[clap(
            name = "no_json_output",
            long = "no-json-output",
            value_name = "NO_JSON_OUTPUT",
            help = "disable generating native JSON file(s) in pyroscope data directory"
        )]
        no_json_output: bool,
        #[clap(
            name = "output_format",
            long = "output-format",
            value_name = "OUTPUT_FORMAT",
            help = "format to export profiling data",
            default_value = "html"
        )]
        output_format: OutputFormat,
        #[clap(
            name = "pid",
            long = "pid",
            value_name = "PID",
            help = "PID of the process you want to profile. Pass -1 to profile the whole system (only supported by ebpfspy)",
            parse(try_from_str),
            default_value = "0"
        )]
        pid: i32,
        #[clap(
            name = "push",
            long = "push",
            value_name = "PUSH",
            help = "use push mode, exposing an ingestion endpoint for the profiled program to use"
        )]
        push: bool,
        #[clap(
            name = "pyspy_blocking",
            long = "pyspy-blocking",
            value_name = "PYSPY_BLOCKING",
            help = "enable blocking mode for pyspy"
        )]
        pyspy_blocking: bool,
        #[clap(
            name = "rbspy_blocking",
            long = "rbspy-blocking",
            value_name = "RBSPY_BLOCKING",
            help = "enable blocking mode for rbspy"
        )]
        rbspy_blocking: bool,
        #[clap(
            name = "sample_rate",
            long = "sample-rate",
            value_name = "SAMPLE_RATE",
            help = "sample rate for the profiler in Hz. 100 means reading 100 times per second",
            default_value = "100"
        )]
        sample_rate: i32,
        #[clap(
            name = "spy_name",
            long = "spy-name",
            value_name = "SPY_NAME",
            help = "name of the profiler to use",
            default_value = "auto"
        )]
        spy_name: Spy,
        #[clap(long, value_name = "URL", help = "URL to gather profiling data from")]
        url: Option<String>,
    },
    #[clap(
            name = "completion",
            about = "Generate the autocompletion script for pyroscope for the specified shell. See each sub-command's help for details on how to use the generated script.",
            long_about = None,
            )]
    Completion {
        #[clap(subcommand)]
        shell: CompletionShell,
    },
    #[clap(
        name = "connect",
        about = "Connect to an existing process and profile it",
        long_about = None,
    )]
    Connect {
        #[clap(
            name = "application_name",
            long = "application-name",
            value_name = "APPLICATION_NAME",
            help = "application name used when uploading profiling data"
        )]
        application_name: Option<String>,
        #[clap(
            name = "auth_token",
            long = "auth-token",
            value_name = "AUTH_TOKEN",
            help = "authorization token used to upload profiling data"
        )]
        auth_token: Option<String>,
        #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process"
        )]
        detect_subprocesses: bool,
        #[clap(
            name = "log_level",
            short,
            long = "log-level",
            value_name = "LOG_LEVEL",
            help = "",
            default_value = "info"
        )]
        log_level: LogLevel,
        #[clap(
            name = "no_logging",
            long = "no-logging",
            value_name = "NO_LOGGING",
            help = "disable logging from pyroscope"
        )]
        no_logging: bool,
        #[clap(
            name = "pid",
            long = "pid",
            value_name = "PID",
            help = "PID of the process you want to profile. Pass -1 to profile the whole system (only supported by ebpfspy)",
            parse(try_from_str)
        )]
        pid: i32,
        #[clap(
            name = "pyspy_blocking",
            long = "pyspy-blocking",
            value_name = "PYSPY_BLOCKING",
            help = "enable blocking mode for pyspy"
        )]
        pyspy_blocking: bool,
        #[clap(
            name = "rbspy_blocking",
            long = "rbspy-blocking",
            value_name = "RBSPY_BLOCKING",
            help = "enable blocking mode for rbspy"
        )]
        rbspy_blocking: bool,
        #[clap(
            name = "sample_rate",
            long = "sample-rate",
            value_name = "SAMPLE_RATE",
            help = "sample rate for the profiler in Hz. 100 means reading 100 times per second",
            default_value = "100"
        )]
        sample_rate: i32,
        #[clap(
            name = "server_address",
            long = "server-address",
            value_name = "SERVER_ADDRESS",
            help = "Pyroscope server address",
            default_value = "http://localhost:4040"
        )]
        server_address: String,
        #[clap(
            name = "spy_name",
            long = "spy-name",
            value_name = "SPY_NAME",
            help = "name of the profiler to use"
        )]
        spy_name: Spy,
        #[clap(
            name = "tag",
            long = "tag",
            value_name = "TAG",
            help = "tag in key=value form. The flag may be specified multiple times"
        )]
        tag: Option<String>,
        #[clap(
            name = "upstream_request_timeout",
            long = "upstream-request-timeout",
            value_name = "UPSTREAM_REQUEST_TIMEOUT",
            help = "profile upload timeout",
            default_value = "10s"
        )]
        upstream_request_timeout: String,
        #[clap(
            name = "upstream_threads",
            long = "upstream-threads",
            value_name = "UPSTREAM_THREADS",
            help = "number of upload threads",
            parse(try_from_str),
            default_value = "4"
        )]
        upstream_threads: u32,
    },
    #[clap(
        name = "exec",
        about = "Start a new process from arguments and profile it",
        long_about = None,
    )]
    Exec {
        /// The command to execute
        #[clap(required = true)]
        command: Option<String>,
        #[clap(
            name = "application_name",
            long = "application-name",
            value_name = "APPLICATION_NAME",
            help = "application name used when uploading profiling data"
        )]
        application_name: Option<String>,
        #[clap(
            name = "auth_token",
            long = "auth-token",
            value_name = "AUTH_TOKEN",
            help = "authorization token used to upload profiling data"
        )]
        auth_token: Option<String>,
        #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process"
        )]
        detect_subprocesses: bool,
        #[clap(
            name = "group_name",
            long = "group-name",
            value_name = "GROUP_NAME",
            help = "start process under specified group name"
        )]
        group_name: Option<String>,
        #[clap(
            name = "log_level",
            short,
            long = "log-level",
            value_name = "LOG_LEVEL",
            help = "",
            default_value = "info"
        )]
        log_level: LogLevel,
        #[clap(
            name = "no_logging",
            long = "no-logging",
            value_name = "NO_LOGGING",
            help = "disable logging from pyroscope"
        )]
        no_logging: bool,
        #[clap(
            name = "no_root_drop",
            long = "no-root-drop",
            value_name = "NO_ROOT_DROP",
            help = "disable permissions drop when ran under root. use this one if you want to run your command as root"
        )]
        no_root_drop: bool,
        #[clap(
            name = "pyspy_blocking",
            long = "pyspy-blocking",
            value_name = "PYSPY_BLOCKING",
            help = "enable blocking mode for pyspy"
        )]
        pyspy_blocking: bool,
        #[clap(
            name = "rbspy_blocking",
            long = "rbspy-blocking",
            value_name = "RBSPY_BLOCKING",
            help = "enable blocking mode for rbspy"
        )]
        rbspy_blocking: bool,
        #[clap(
            name = "sample_rate",
            long = "sample-rate",
            value_name = "SAMPLE_RATE",
            help = "sample rate for the profiler in Hz. 100 means reading 100 times per second",
            default_value = "100"
        )]
        sample_rate: u32,
        #[clap(
            name = "server_address",
            long = "server-address",
            value_name = "SERVER_ADDRESS",
            help = "Pyroscope server address",
            default_value = "http://localhost:4040"
        )]
        server_address: String,
        #[clap(
            name = "spy_name",
            long = "spy-name",
            value_name = "SPY_NAME",
            help = "name of the profiler to use"
        )]
        spy_name: Spy,
        #[clap(
            name = "tag",
            long = "tag",
            value_name = "TAG",
            help = "tag in key=value form. The flag may be specified multiple times"
        )]
        tag: Option<String>,
        #[clap(
            name = "upstream_request_timeout",
            long = "upstream-request-timeout",
            value_name = "UPSTREAM_REQUEST_TIMEOUT",
            help = "profile upload timeout",
            default_value = "10s"
        )]
        upstream_request_timeout: String,
        #[clap(
            name = "upstream_threads",
            long = "upstream-threads",
            value_name = "UPSTREAM_THREADS",
            help = "number of upload threads",
            parse(try_from_str),
            default_value = "4"
        )]
        upstream_threads: u32,
        #[clap(
            name = "user_name",
            long = "user-name",
            value_name = "USER_NAME",
            help = "start process under specified user name"
        )]
        user_name: Option<String>,
    },
    #[clap(
        name = "config",
        about = "Show Configuration",
        long_about = None,
    )]
    Config,
}

/// Supported Completion Shells
#[derive(Subcommand, PartialEq, Debug)]
enum CompletionShell {
    #[clap(about = "generate the autocompletion script for bash")]
    Bash,
    #[clap(about = "generate the autocompletion script for fish")]
    Fish,
    #[clap(about = "generate the autocompletion script for powershell")]
    Powershell,
    #[clap(about = "generate the autocompletion script for zsh")]
    Zsh,
}

/// Match the command line arguments and run the appropriate command
pub fn cli_match() -> Result<()> {
    // Parse the command line arguments
    let cli = Cli::parse();

    // Merge clap config file if the value is set
    AppConfig::merge_config(cli.config.as_deref())?;

    let app = Cli::into_app();
    AppConfig::merge_args(app)?;

    let mut app = Cli::into_app();

    // Execute the subcommand
    match &cli.command {
        Commands::Adhoc { .. } => {
            commands::adhoc()?;
        }
        Commands::Exec { server_address, .. } => {
            commands::exec()?;
        }
        Commands::Connect { .. } => {
            commands::connect()?;
        }
        Commands::Completion { shell } => match shell {
            CompletionShell::Bash => {
                generate(Bash, &mut app, "pyroscope-cli", &mut std::io::stdout());
            }
            CompletionShell::Fish => {
                generate(Fish, &mut app, "pyroscope-cli", &mut std::io::stdout());
            }
            CompletionShell::Powershell => {
                generate(
                    PowerShell,
                    &mut app,
                    "pyroscope-cli",
                    &mut std::io::stdout(),
                );
            }
            CompletionShell::Zsh => {
                generate(Zsh, &mut app, "pyroscope-cli", &mut std::io::stdout());
            }
        },
        Commands::Config => {
            commands::config()?;
        }
    }

    Ok(())
}
