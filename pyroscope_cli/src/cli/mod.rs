use crate::utils::types::{LogLevel, Spy};
use clap::Args;
use clap::{Parser, Subcommand};
use serde::Deserialize;

const DEFAULT_SERVER_ADDRESS: &str = "http://localhost:4040";

#[derive(Debug, Args, Clone, Deserialize)]
pub struct CommandArgs {
    // required or with defaults
    #[clap(
        required = true,
        name = "application_name",
        long = "application-name",
        value_name = "APPLICATION_NAME",
        help = "application name used when uploading profiling data"
    )]
    pub application_name: String, // todo add flameql validation
    #[clap(
        name = "server_address",
        long = "server-address",
        value_name = "SERVER_ADDRESS",
        help = "[default: http://localhost:4040] Pyroscope server address",
        default_value = DEFAULT_SERVER_ADDRESS
    )]
    pub server_address: String,
    #[clap(
        name = "sample_rate",
        long = "sample-rate",
        value_name = "SAMPLE_RATE",
        help = "[default: 100] sample rate for the profiler in Hz. 100 means reading 100 times per second",
        default_value = "100"
    )]
    pub sample_rate: u32,
    #[clap(
        required = true,
        value_enum,
        name = "spy_name",
        long = "spy-name",
        value_name = "SPY_NAME",
        help = "name of the profiler to use"
    )]
    pub spy_name: Spy, // todo consider make it subcommand
    #[clap(
        value_enum,
        name = "log_level",
        short,
        long = "log-level",
        value_name = "LOG_LEVEL",
        help = "[default: error] log level for the application",
        default_value = "error"
    )]
    pub log_level: LogLevel,
    // optional
    #[clap(
        name = "auth_token",
        long = "auth-token",
        value_name = "AUTH_TOKEN",
        help = "Authentication token used when uploading profiling data"
    )]
    pub auth_token: Option<String>,
    #[clap(
        name = "basic_auth_username",
        long = "basic-auth-username",
        value_name = "BASIC_AUTH_USERNAME",
        help = "HTTP Basic Authentication username used when uploading profiling data"
    )]
    pub basic_auth_username: Option<String>,
    #[clap(
        name = "basic_auth_password",
        long = "basic-auth-password",
        value_name = "BASIC_AUTH_PASSWORD",
        help = "HTTP Basic Authentication password used when uploading profiling data"
    )]
    pub basic_auth_password: Option<String>,
    #[clap(
        name = "tenant_id",
        long = "tenant-id",
        value_name = "TENANT_ID",
        help = "X-Scope-OrgID header for phlare multi-tenancy"
    )]
    pub tenant_id: Option<String>,
    #[clap(
            name = "detect_subprocesses",
            long = "detect-subprocesses",
            value_name = "DECTECT_SUBPROCESSES",
            help = "keep track of and profile subprocesses of the main process",
            action = clap::ArgAction::SetTrue,
            default_value = "false",
    )]
    pub detect_subprocesses: bool,

    #[clap(
            name = "blocking",
            long = "blocking",
            value_name = "BLOCKING",
            help = "enable blocking mode. [supported by: rbspy, pyspy]",
            action = clap::ArgAction::SetTrue,
            default_value = "false",
    )]
    pub blocking: bool,
    #[clap(
            name = "oncpu",
            long = "oncpu",
            value_name = "ONCPU",
            help = "enable oncpu mode. [supported by: rbspy, pyspy]",
            action = clap::ArgAction::SetTrue,
            default_value = "true",
    )]
    pub oncpu: bool,
    #[clap(
            name = "pyspy_gil",
            long = "pyspy-gil",
            value_name = "PYSPY_GIL",
            help = "enable GIL mode for pyspy",
            action = clap::ArgAction::SetTrue,
            default_value = "true",
    )]
    pub pyspy_gil: bool,
    #[clap(
        name = "tag",
        long = "tag",
        value_name = "TAG",
        help = "tag in key=value form. The flag may be specified multiple times"
    )]
    pub tag: Option<Vec<String>>,
    #[clap(
        name = "http_header",
        long = "http_header",
        value_name = "HTTP_HEADER",
        help = "http header in 'X-Header=HeaderValue' form. The flag may be specified multiple times"
    )]
    pub http_header: Option<Vec<String>>,
    #[clap(
        name = "user_name",
        long = "user-name",
        value_name = "USER_NAME",
        help = "start process under specified user name"
    )]
    pub user_name: Option<u32>,
    #[clap(
        name = "group_name",
        long = "group-name",
        value_name = "GROUP_NAME",
        help = "start process under specified group name"
    )]
    pub group_name: Option<u32>,
}

#[derive(Parser, Debug)]
#[clap(
    name = "pyroscope-cli",
    author,
    about,
    long_about = "Pyroscope CLI",
    version
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Args)]
pub struct Connect {
    #[clap(
        required = true,
        name = "pid",
        long = "pid",
        value_name = "PID",
        help = "PID of the process you want to profile. Pass -1 to profile the whole system (only supported by ebpfspy)"
    )]
    pub pid: i32,

    #[command(flatten)]
    pub common: CommandArgs,
}

#[derive(Debug, Args)]
pub struct Exec {
    #[clap(
        required = true,
        name = "command",
        value_name = "COMMAND",
        help = "command to execute"
    )]
    pub command: Vec<String>,

    #[command(flatten)]
    pub common: CommandArgs,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[clap(
        name = "connect",
        about = "Connect to an existing process and profile it",
        long_about = None,
    )]
    Connect(Connect),

    #[clap(
        name = "exec",
        about = "Start a new process from arguments and profile it",
        long_about = None,
    )]
    Exec(Exec),
}
