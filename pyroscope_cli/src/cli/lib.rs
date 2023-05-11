use clap::{AppSettings, IntoApp, Parser, Subcommand};
use clap_complete::{
    generate,
    shells::{Bash, Fish, PowerShell, Zsh},
};
use std::path::PathBuf;

use crate::core::commands;
use crate::utils::{
    app_config::AppConfig,
    error::Result,
    types::{LogLevel, Spy},
};

#[derive(Parser, Debug)]
#[clap(
    name = "pyroscope-cli",
    author,
    about,
    long_about = "Pyroscope CLI",
    version
)]
#[clap(global_setting(AppSettings::DeriveDisplayOrder))]
pub struct Cli {
    /// Set a custom config file
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Subcommands
    #[clap(subcommand)]
    command: Commands,
}

/// Clap Derive for pyroscope-cli commands
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
        #[clap(
            name = "auth_token",
            long = "auth-token",
            value_name = "AUTH_TOKEN",
            help = "Authentication token used when uploading profiling data"
        )]
        auth_token: Option<String>,
        #[clap(
            name = "basic_auth_username",
            long = "basic-auth-username",
            value_name = "BASIC_AUTH_USERNAME",
            help = "HTTP Basic Authentication username used when uploading profiling data"
        )]
        basic_auth_username: Option<String>,
        #[clap(
            name = "basic_auth_password",
            long = "basic-auth-password",
            value_name = "BASIC_AUTH_PASSWORD",
            help = "HTTP Basic Authentication password used when uploading profiling data"
        )]
        basic_auth_password: Option<String>,
        #[clap(
            name = "scope_org_id",
            long = "scope-org-id",
            value_name = "SCOPE_ORG_ID",
            help = "X-Scope-OrgID header for phlare multi-tenancy"
        )]
        scope_org_id: Option<String>,
        #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process",
            takes_value = false
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
            name = "pid",
            long = "pid",
            value_name = "PID",
            help = "PID of the process you want to profile. Pass -1 to profile the whole system (only supported by ebpfspy)",
            parse(try_from_str)
        )]
        pid: i32,
        #[clap(
            name = "blocking",
            long = "blocking",
            value_name = "BLOCKING",
            help = "enable blocking mode. [supported by: rbspy, pyspy]",
            takes_value = false
        )]
        blocking: bool,
        #[clap(
            name = "oncpu",
            long = "oncpu",
            value_name = "ONCPU",
            help = "enable oncpu mode. [supported by: rbspy, pyspy]",
            takes_value = false
        )]
        pyspy_idle: bool,
        #[clap(
            name = "pyspy_gil",
            long = "pyspy-gil",
            value_name = "PYSPY_GIL",
            help = "enable GIL mode for pyspy",
            takes_value = false
        )]
        pyspy_gil: bool,
        #[clap(
            name = "pyspy_native",
            long = "pyspy-native",
            value_name = "PYSPY_NATIVE",
            help = "enable native extensions profiling for pyspy",
            takes_value = false
        )]
        pyspy_native: bool,
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
            multiple_occurrences = true,
            name = "tag",
            long = "tag",
            value_name = "TAG",
            help = "tag in key=value form. The flag may be specified multiple times"
        )]
        tag: Option<String>,
        #[clap(
            multiple_occurrences = true,
            name = "http_header",
            long = "http_header",
            value_name = "HTTP_HEADER",
            help = "http header in 'X-Header: HeaderValue' form. The flag may be specified multiple times"
        )]
        http_header: Option<String>,
    },

    #[clap(
        name = "exec",
        about = "Start a new process from arguments and profile it",
        long_about = None,
    )]
    Exec {
        #[clap(
            required = true,
            name = "command",
            value_name = "COMMAND",
            help = "command to execute",
            takes_value = true,
            multiple_values = true
        )]
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
            help = "Authentication token used when uploading profiling data"
        )]
        auth_token: Option<String>,
        #[clap(
            name = "basic_auth_username",
            long = "basic-auth-username",
            value_name = "BASIC_AUTH_USERNAME",
            help = "HTTP Basic Authentication username used when uploading profiling data"
        )]
        basic_auth_username: Option<String>,
        #[clap(
            name = "basic_auth_password",
            long = "basic-auth-password",
            value_name = "BASIC_AUTH_PASSWORD",
            help = "HTTP Basic Authentication password used when uploading profiling data"
        )]
        basic_auth_password: Option<String>,
        #[clap(
            name = "scope_org_id",
            long = "scope-org-id",
            value_name = "SCOPE_ORG_ID",
            help = "X-Scope-OrgID header for phlare multi-tenancy"
        )]
        scope_org_id: Option<String>,
        #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process",
            takes_value = false
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
            name = "blocking",
            long = "blocking",
            value_name = "BLOCKING",
            help = "enable blocking mode. [supported by: rbspy, pyspy]",
            takes_value = false
        )]
        blocking: bool,
        #[clap(
            name = "oncpu",
            long = "oncpu",
            value_name = "ONCPU",
            help = "enable oncpu mode. [supported by: rbspy, pyspy]",
            takes_value = false
        )]
        pyspy_idle: bool,
        #[clap(
            name = "pyspy_gil",
            long = "pyspy-gil",
            value_name = "PYSPY_GIL",
            help = "enable GIL mode for pyspy",
            takes_value = false
        )]
        pyspy_gil: bool,
        #[clap(
            name = "pyspy_native",
            long = "pyspy-native",
            value_name = "PYSPY_NATIVE",
            help = "enable native extensions profiling for pyspy",
            takes_value = false
        )]
        pyspy_native: bool,
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
            multiple_occurrences = true,
            name = "tag",
            long = "tag",
            value_name = "TAG",
            help = "tag in key=value form. The flag may be specified multiple times"
        )]
        tag: Option<String>,
        #[clap(
            multiple_occurrences = true,
            name = "http_header",
            long = "http_header",
            value_name = "HTTP_HEADER",
            help = "http header in 'X-Header: HeaderValue' form. The flag may be specified multiple times"
            )]
        http_header: Option<String>,
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

    let app = Cli::command();

    // Merge clap args into config
    AppConfig::merge_args(app)?;

    let mut app = Cli::command();

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
    }

    Ok(())
}
