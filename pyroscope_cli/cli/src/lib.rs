use clap::{AppSettings, IntoApp, Parser, Subcommand};
use clap_complete::{
    generate,
    shells::{Bash, Fish, PowerShell, Zsh},
};
use std::path::PathBuf;

use core::commands;
use utils::app_config::AppConfig;
use utils::error::Result;
use utils::types::{LogLevel, Spy};

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
        // TODO: placeholder for future implementation
        //#[clap(
        //name = "auth_token",
        //long = "auth-token",
        //value_name = "AUTH_TOKEN",
        //help = "authorization token used to upload profiling data"
        //)]
        //auth_token: Option<String>,
        #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process"
        )]
        detect_subprocesses: bool,
        #[clap(
            arg_enum,
            name = "log_level",
            short,
            long = "log-level",
            value_name = "LOG_LEVEL",
            help = "[default: info] log level for the application"
        )]
        log_level: Option<LogLevel>,
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
            help = "[default: 100] sample rate for the profiler in Hz. 100 means reading 100 times per second"
        )]
        sample_rate: Option<u32>,
        #[clap(
            name = "server_address",
            long = "server-address",
            value_name = "SERVER_ADDRESS",
            help = "[default: http://localhost:4040] Pyroscope server address"
        )]
        server_address: Option<String>,
        #[clap(
            arg_enum,
            name = "spy_name",
            long = "spy-name",
            value_name = "SPY_NAME",
            help = "name of the profiler to use"
        )]
        spy_name: Spy,
        #[clap(
            multiple = true,
            name = "tag",
            long = "tag",
            value_name = "TAG",
            help = "tag in key=value form. The flag may be specified multiple times"
        )]
        tag: Option<String>,
        // TODO: placeholder for future implementation
        //#[clap(
        //name = "upstream_request_timeout",
        //long = "upstream-request-timeout",
        //value_name = "UPSTREAM_REQUEST_TIMEOUT",
        //help = "profile upload timeout",
        //default_value = "10s"
        //)]
        //upstream_request_timeout: String,
        //#[clap(
        //name = "upstream_threads",
        //long = "upstream-threads",
        //value_name = "UPSTREAM_THREADS",
        //help = "number of upload threads",
        //parse(try_from_str),
        //default_value = "4"
        //)]
        //upstream_threads: u32,
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
        // TODO: placeholder for future implementation
        //#[clap(
        //name = "auth_token",
        //long = "auth-token",
        //value_name = "AUTH_TOKEN",
        //help = "authorization token used to upload profiling data"
        //)]
        //auth_token: Option<String>,
        #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process"
        )]
        detect_subprocesses: bool,
        #[clap(
            arg_enum,
            name = "log_level",
            short,
            long = "log-level",
            value_name = "LOG_LEVEL",
            help = "[default: info] log level for the application"
        )]
        log_level: Option<LogLevel>,
        #[clap(
            name = "no_logging",
            long = "no-logging",
            value_name = "NO_LOGGING",
            help = "disable logging from pyroscope"
        )]
        no_logging: bool,
        //#[clap(
        //name = "no_root_drop",
        //long = "no-root-drop",
        //value_name = "NO_ROOT_DROP",
        //help = "disable permissions drop when ran under root. use this one if you want to run your command as root"
        //)]
        //no_root_drop: bool,
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
            help = "[default: 100] sample rate for the profiler in Hz. 100 means reading 100 times per second"
        )]
        sample_rate: Option<u32>,
        #[clap(
            name = "server_address",
            long = "server-address",
            value_name = "SERVER_ADDRESS",
            help = "[default: http://localhost:4040] Pyroscope server address"
        )]
        server_address: Option<String>,
        #[clap(
            arg_enum,
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
        // TODO: placeholder for future implementation
        //#[clap(
        //name = "upstream_request_timeout",
        //long = "upstream-request-timeout",
        //value_name = "UPSTREAM_REQUEST_TIMEOUT",
        //help = "profile upload timeout",
        //default_value = "10s"
        //)]
        //upstream_request_timeout: String,
        //#[clap(
        //name = "upstream_threads",
        //long = "upstream-threads",
        //value_name = "UPSTREAM_THREADS",
        //help = "number of upload threads",
        //parse(try_from_str),
        //default_value = "4"
        //)]
        //upstream_threads: u32,
        #[clap(
            name = "user_name",
            long = "user-name",
            value_name = "USER_NAME",
            help = "start process under specified user name"
        )]
        user_name: Option<String>,
        #[clap(
            name = "group_name",
            long = "group-name",
            value_name = "GROUP_NAME",
            help = "start process under specified group name"
        )]
        group_name: Option<String>,
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

    // Merge clap args into config
    AppConfig::merge_args(app)?;

    let mut app = Cli::into_app();
    // Execute the subcommand
    match &cli.command {
        Commands::Exec { .. } => {
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
