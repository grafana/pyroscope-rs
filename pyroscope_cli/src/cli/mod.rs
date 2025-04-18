use crate::utils::error::{Error, Result};
use crate::utils::types::{LogLevel, Spy};
use clap::error::ErrorKind::MissingRequiredArgument;
use clap::{Args, Command, FromArgMatches};
use clap::{CommandFactory, Parser, Subcommand};
use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::PathBuf;
use clap::builder::Str;

const DEFAULT_SERVER_ADDRESS: &str = "http://localhost:4040";

#[derive(Debug, Args, Clone, Deserialize)]
struct CommandArgs {
    // required or with defaults
    #[clap(
        name = "application_name",
        long = "application-name",
        value_name = "APPLICATION_NAME",
        help = "application name used when uploading profiling data"
    )]
    pub application_name: Option<String>, // todo add flameql validation
    #[clap(
        name = "server_address",
        long = "server-address",
        value_name = "SERVER_ADDRESS",
        help = "[default: http://localhost:4040] Pyroscope server address",
        default_value = DEFAULT_SERVER_ADDRESS
    )]
    pub server_address: Option<String>,
    #[clap(
        name = "sample_rate",
        long = "sample-rate",
        value_name = "SAMPLE_RATE",
        help = "[default: 100] sample rate for the profiler in Hz. 100 means reading 100 times per second",
        default_value = "100"
    )]
    pub sample_rate: Option<u32>,
    #[clap(
        value_enum,
        name = "spy_name",
        long = "spy-name",
        value_name = "SPY_NAME",
        help = "name of the profiler to use"
    )]
    pub spy_name: Option<Spy>,
    #[clap(
        value_enum,
        name = "log_level",
        short,
        long = "log-level",
        value_name = "LOG_LEVEL",
        help = "[default: error] log level for the application",
        default_value = "error"
    )]
    pub log_level: Option<LogLevel>,
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
            action = clap::ArgAction::SetTrue
    )]
    pub detect_subprocesses: Option<bool>,

    #[clap(
            name = "blocking",
            long = "blocking",
            value_name = "BLOCKING",
            help = "enable blocking mode. [supported by: rbspy, pyspy]",
            action = clap::ArgAction::SetTrue
    )]
    pub blocking: Option<bool>,
    #[clap(
            name = "oncpu",
            long = "oncpu",
            value_name = "ONCPU",
            help = "enable oncpu mode. [supported by: rbspy, pyspy]",
            action = clap::ArgAction::SetTrue,
            default_value = "true",
    )]
    pub oncpu: Option<bool>,
    #[clap(
            name = "pyspy_gil",
            long = "pyspy-gil",
            value_name = "PYSPY_GIL",
            help = "enable GIL mode for pyspy",
            action = clap::ArgAction::SetTrue,
            default_value = "true",
    )]
    pub pyspy_gil: Option<bool>,
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
struct Cli {
    #[clap(short, long)]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Args)]
struct Connect {
    #[clap(
        name = "pid",
        long = "pid",
        value_name = "PID",
        help = "PID of the process you want to profile. Pass -1 to profile the whole system (only supported by ebpfspy)"
    )]
    pub pid: Option<i32>,

    #[command(flatten)]
    pub common: CommandArgs,
}

#[derive(Debug, Args)]
struct Exec {
    #[clap(name = "command", value_name = "COMMAND", help = "command to execute")]
    pub command: Option<Vec<String>>,

    #[command(flatten)]
    pub common: CommandArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
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


pub struct ValidatedConnect {
    pub pid: i32,
}

pub struct ValidatedExec {
    pub command: Vec<String>,
}

pub enum ValidatedCommands {
    Connect(ValidatedConnect),
    Exec(ValidatedExec),
}

pub struct ValidatedArgs {
    // required or with defaults
    pub application_name: String,
    pub server_address: String,
    pub sample_rate: u32,
    pub spy_name: Spy,
    pub log_level: LogLevel,
    // optional
    pub auth_token: Option<String>,
    pub basic_auth_username: Option<String>,
    pub basic_auth_password: Option<String>,
    pub tenant_id: Option<String>,
    pub detect_subprocesses: bool,
    pub blocking: bool,
    pub oncpu: bool,
    pub pyspy_gil: bool,
    pub tag: Option<Vec<String>>,
    pub http_header: Option<Vec<String>>,
    pub user_name: Option<u32>,
    pub group_name: Option<u32>,
}


fn validate_args(args: CommandArgs) -> std::result::Result<ValidatedArgs, MissingFields> {
    let mut missing_args = vec![];
    // fn make_required_arg(name: &str, cmd: Command) -> Command {
    //     cmd.mut_subcommand("exec", |x| x.mut_arg(name, |a| a.required(true)))
    //         .mut_subcommand("connect", |x| x.mut_arg(name, |a| a.required(true)))
    // }
    let application_name = match &args.application_name {
        None => {
            missing_args.push("application_name");
            ""
        }
        Some(s) => s,
    };
    let server_address = match &args.server_address {
        None => {
            missing_args.push("server_address");
            ""
        }
        Some(s) => s,
    };
    let sample_rate = match args.sample_rate {
        None => {
            missing_args.push("sample_rate");
            0
        }
        Some(s) => s,
    };
    let spy_name = match args.spy_name {
        None => {
            missing_args.push("spy_name");
            Spy::Pyspy
        }
        Some(s) => s,
    };
    let log_level = match args.log_level {
        None => {
            missing_args.push("log_level");
            LogLevel::Debug
        }
        Some(s) => s,
    };
    if !missing_args.is_empty() {
        return Err(missing_args);
        // for name in &missing_args {
        //     cmd_proto = cmd_proto
        //         .mut_subcommand("exec", |x| x.mut_arg(name, |a| a.required(true)))
        //         .mut_subcommand("connect", |x| x.mut_arg(name, |a| a.required(true)))
        // }
        // let mut matches = cmd_proto.clone().get_matches();
        // match Cli::from_arg_matches_mut(&mut matches).map_err(|err| err.format(&mut cmd_proto)) {
        //     Ok(_) => {
        //         panic!("TODO") // should not happend
        //     }
        //     Err(e) => e.exit(),
        // };
    }


    Ok(ValidatedArgs {
        application_name: application_name.to_string(),
        server_address: server_address.to_string(),
        sample_rate,
        spy_name,
        log_level,
        auth_token: args.auth_token.clone(),
        basic_auth_username: args.basic_auth_username.clone(),
        basic_auth_password: args.basic_auth_password.clone(),
        tenant_id: args.tenant_id.clone(),
        detect_subprocesses: args.detect_subprocesses.unwrap_or(false),
        blocking: args.blocking.unwrap_or(false),
        oncpu: args.oncpu.unwrap_or(false),
        pyspy_gil: args.pyspy_gil.unwrap_or(false),
        tag: args.tag.clone(),
        http_header: args.http_header.clone(),
        user_name: args.user_name,
        group_name: args.group_name,
    })
}

// todo put behind a feature and no config merging by default
pub fn parse() -> Result<(ValidatedCommands, ValidatedArgs)> {
    let cli = Cli::parse();
    let mut missing_args: Vec<&str> = vec![];

    if let Some(config) = &cli.config {
        let toml_args = read_toml_args(config)?;
        let cmd_proto = || cmd_with_defaults(&toml_args);
        let mut matches = cmd_proto().get_matches();
        let cli = match Cli::from_arg_matches_mut(&mut matches)
            .map_err(|err| err.format(&mut cmd_proto()))
        {
            Ok(cli) => cli,
            Err(e) => e.exit(),
        };
        
        validate(cli, cmd_proto())
        
    } else {
        validate(cli, Cli::command())
    }
}

fn validate(cli: Cli, mut failed_cmd_proto : Command) -> Result<(ValidatedCommands, ValidatedArgs)>  {

    let cmd = validate_command(&cli.command);
    let args = validate_args(command_args(cli.command));
    
    fn require_arg(mut cmd :Command, name: &str) -> Command {
        cmd = cmd.mut_subcommand("exec", |cmd| {
            cmd.mut_args(|a| {
                match a.get_value_names() {
                    None => {
                        a
                    }
                    Some(names) => {
                        if names[0].as_str() == name {
                            a.required(true)
                        } else {
                            a
                        }
                    }
                }
            })
        });
        cmd = cmd.mut_subcommand("connect", |cmd| {
            cmd.mut_args(|a| {
                match a.get_value_names() {
                    None => {
                        a
                    }
                    Some(names) => {
                        if names[0].as_str() == name {
                            a.required(true)
                        } else {
                            a
                        }
                    }
                }
            })
        });
        cmd
    }
    match (cmd, args) {
        (Ok(cmd), Ok(args)) => {
            return Ok((cmd, args));
        }
        (Err(args), Ok(_)) => {
            for a in args {
                failed_cmd_proto = require_arg(failed_cmd_proto, a)
            }
        }
        (Ok(_), Err(args)) => {
            for a in args {
                failed_cmd_proto = require_arg(failed_cmd_proto, a)
            }
        }
        (Err(args1), Err(args)) => {
            for a in args {
                failed_cmd_proto = require_arg(failed_cmd_proto, a)
            }
            for a in args1 {
                failed_cmd_proto = require_arg(failed_cmd_proto, a)
            }
        }
    };
    let mut matches = failed_cmd_proto.clone().get_matches();
    match Cli::from_arg_matches_mut(&mut matches).map_err(|err| err.format(&mut failed_cmd_proto)) {
        Ok(_) => {
            panic!("TODO") // should not happend
        }
        Err(e) => e.exit(),
    };
}

type MissingFields = Vec<&'static str>;

fn cmd_with_defaults(toml_args: &CommandArgs) -> Command {
    let mut cmd = Cli::command();
    // cmd = update_default_from_toml(cmd, "application_name", toml_args.application_name);
    cmd = update_default_from_toml(cmd, "application_name", toml_args.application_name.clone());
    cmd = update_default_from_toml(cmd, "server_address", toml_args.server_address.clone());
    cmd = update_default_from_toml(
        cmd,
        "sample_rate",
        toml_args.sample_rate.map(|x| x.to_string()),
    );
    cmd = update_default_from_toml(cmd, "spy_name", toml_args.spy_name);
    cmd = update_default_from_toml(cmd, "log_level", toml_args.log_level);
    cmd = update_default_from_toml(cmd, "auth_token", toml_args.auth_token.clone());
    cmd = update_default_from_toml(
        cmd,
        "basic_auth_username",
        toml_args.basic_auth_username.clone(),
    );
    cmd = update_default_from_toml(
        cmd,
        "basic_auth_password",
        toml_args.basic_auth_password.clone(),
    );
    cmd = update_default_from_toml(cmd, "tenant_id", toml_args.tenant_id.clone());
    cmd = update_default_from_toml(
        cmd,
        "detect_subprocesses",
        toml_args.detect_subprocesses.map(|x| x.to_string()),
    );
    cmd = update_default_from_toml(
        cmd,
        "blocking",
        toml_args.blocking.clone().map(|x| x.to_string()),
    );
    cmd = update_default_from_toml(cmd, "oncpu", toml_args.oncpu.clone().map(|x| x.to_string()));
    cmd = update_default_from_toml(
        cmd,
        "pyspy_gil",
        toml_args.clone().pyspy_gil.map(|x| x.to_string()),
    );
    // cmd = update_default_from_toml(cmd, "tag", toml_args.tag.map(|x| x.to_string()));
    // cmd = update_default_from_toml(
    //     cmd,
    //     "http_header",
    //     toml_args.http_header.map(|x| x.to_string()),
    // );
    cmd = update_default_from_toml(cmd, "user_name", toml_args.user_name.map(|x| x.to_string()));
    cmd = update_default_from_toml(
        cmd,
        "group_name",
        toml_args.group_name.map(|x| x.to_string()),
    );
    cmd
}

fn update_default_from_toml<T: AsRef<str>>(cmd: Command, name: &str, def: Option<T>) -> Command {
    match def {
        None => cmd,
        Some(v) => cmd
            .mut_subcommand("exec", |x| {
                x.mut_arg(name, |a| a.default_value(v.as_ref().to_string()))
            })
            .mut_subcommand("connect", |x| {
                x.mut_arg(name, |a| a.default_value(v.as_ref().to_string()))
            }),
    }
}


fn command_args(command: Commands) -> CommandArgs {
    match command {
        Commands::Connect(connect) => connect.common,
        Commands::Exec(exec) => exec.common,
    }
}

fn validate_command(command: &Commands) -> std::result::Result<ValidatedCommands, MissingFields> {
    match command {
        Commands::Connect(connect) => match connect.pid {
            None => return Err(vec!["pid"]),
            Some(pid) => Ok(ValidatedCommands::Connect {
                0: ValidatedConnect { pid },
            }),
        },
        Commands::Exec(exec) => {
            match &exec.command {
                None => return Err(vec!["command"]),
                Some(command) => {
                    Ok(ValidatedCommands::Exec {
                        0: ValidatedExec {
                            command: exec.command.clone().unwrap_or(vec!["qwe".to_string()]), /* todo */
                        },
                    })
                }
            }
        }
    }
}

fn read_toml_args(f: &PathBuf) -> Result<CommandArgs> {
    let mut f = OpenOptions::new().read(true).open(f)?;
    let mut buf = Vec::with_capacity(f.metadata()?.len() as usize);
    f.read_to_end(&mut buf)?;
    let buf = String::from_utf8(buf)
        .map_err(|err| Error::new_with_source("toml config utf conversion error", err))?;

    let res = toml::from_str(&buf)
        .map_err(|err| Error::new_with_source("toml config parsing error", err))?;
    Ok(res)
}
