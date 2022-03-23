use config::{Config, Environment, Source};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::path::Path;
use std::sync::RwLock;

use super::error::Result;
use crate::types::{LogLevel, Spy};

// CONFIG static variable. It's actually an AppConfig
// inside an RwLock.
lazy_static! {
    pub static ref CONFIG: RwLock<Config> = RwLock::new(Config::new());
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub debug: bool,
    pub pid: Option<i32>,
    pub spy_name: Spy,
    pub application_name: Option<String>,
    pub detect_subprocesses: Option<bool>,
    pub no_logging: Option<bool>,
    pub log_level: LogLevel,
    pub no_root_drop: Option<bool>,
    pub rbspy_blocking: Option<bool>,
    pub pyspy_blocking: Option<bool>,
    pub pyspy_idle: Option<bool>,
    pub pyspy_gil: Option<bool>,
    pub pyspy_native: Option<bool>,
    pub sample_rate: Option<u32>,
    pub server_address: Option<String>,
    pub tag: Option<String>,
    // TODO: placeholder for future implementation
    //pub upstream_request_timeout: Option<String>,
    //pub upstream_threads: Option<u32>,
    //pub auth_token: Option<String>,
    pub user_name: Option<u32>,
    pub group_name: Option<u32>,
    pub command: Option<String>,
}

impl AppConfig {
    /// Initialize AppConfig.
    pub fn init(default_config: Option<&str>) -> Result<()> {
        let mut settings = Config::new();

        // Embed file into executable
        // This macro will embed the configuration file into the
        // executable. Check include_str! for more info.
        if let Some(config_contents) = default_config {
            //let contents = include_str!(config_file_path);
            settings.merge(config::File::from_str(
                config_contents,
                config::FileFormat::Toml,
            ))?;
        }

        // Merge settings with env variables
        settings.merge(Environment::with_prefix("PYROSCOPE"))?;

        // Save Config to RwLoc
        {
            let mut w = CONFIG.write()?;
            *w = settings;
        }

        Ok(())
    }

    pub fn merge_args(app: clap::App) -> Result<()> {
        let args = app.get_matches();

        // Connect Command
        if let Some(sub_connect) = args.subcommand_matches("connect") {
            if sub_connect.is_present("log_level") {
                if let Some(log_level) = sub_connect.value_of("log_level") {
                    AppConfig::set("log_level", log_level)?;
                }
            }
            if sub_connect.is_present("no_logging") {
                if let Some(no_logging) = sub_connect.value_of("no_logging") {
                    AppConfig::set("no_logging", no_logging)?;
                }
            }
            if sub_connect.is_present("pyspy_blocking") {
                if let Some(pyspy_blocking) = sub_connect.value_of("pyspy_blocking") {
                    AppConfig::set("pyspy_blocking", pyspy_blocking)?;
                }
            }
            if sub_connect.is_present("rbspy_blocking") {
                if let Some(rbspy_blocking) = sub_connect.value_of("rbspy_blocking") {
                    AppConfig::set("rbspy_blocking", rbspy_blocking)?;
                }
            }
            if sub_connect.is_present("sample_rate") {
                if let Some(sample_rate) = sub_connect.value_of("sample_rate") {
                    AppConfig::set("sample_rate", sample_rate)?;
                }
            }
            if sub_connect.is_present("server_address") {
                if let Some(server_address) = sub_connect.value_of("server_address") {
                    AppConfig::set("server_address", server_address)?;
                }
            }
            if sub_connect.is_present("tag") {
                if let Some(tags) = sub_connect.values_of("tag") {
                    // Join tags by ;
                    let tag: String = tags.collect::<Vec<&str>>().join(";");

                    AppConfig::set("tag", tag.as_str())?;
                }
            }
            // TODO: placeholder for future implementation
            //if sub_connect.is_present("upstream_request_timeout") {
            //if let Some(upstream_request_timeout) =
            //sub_connect.value_of("upstream_request_timeout")
            //{
            //AppConfig::set("upstream_request_timeout", upstream_request_timeout)?;
            //}
            //}
            //if sub_connect.is_present("upstream_threads") {
            //if let Some(upstream_threads) = sub_connect.value_of("upstream_threads") {
            //AppConfig::set("upstream_threads", upstream_threads)?;
            //}
            //}
            if sub_connect.is_present("application_name") {
                if let Some(application_name) = sub_connect.value_of("application_name") {
                    AppConfig::set("application_name", application_name)?;
                }
            }
            // TODO: placeholder for future implementation
            //if sub_connect.is_present("auth_token") {
            //if let Some(auth_token) = sub_connect.value_of("auth_token") {
            //AppConfig::set("auth_token", auth_token)?;
            //}
            //}
            if sub_connect.is_present("detect_subprocesses") {
                if let Some(detect_subprocesses) = sub_connect.value_of("detect_subprocesses") {
                    AppConfig::set("detect_subprocesses", detect_subprocesses)?;
                }
            }
            if sub_connect.is_present("pid") {
                if let Some(pid) = sub_connect.value_of("pid") {
                    AppConfig::set("pid", pid)?;
                }
            }
            if sub_connect.is_present("spy_name") {
                if let Some(spy_name) = sub_connect.value_of("spy_name") {
                    AppConfig::set("spy_name", spy_name)?;
                }
            }
        }

        // Exec Command
        if let Some(sub_exec) = args.subcommand_matches("exec") {
            if sub_exec.is_present("command") {
                if let Some(command) = sub_exec.value_of("command") {
                    AppConfig::set("command", command)?;
                }
            }
            if sub_exec.is_present("log_level") {
                if let Some(log_level) = sub_exec.value_of("log_level") {
                    AppConfig::set("log_level", log_level)?;
                }
            }
            if sub_exec.is_present("no_logging") {
                if let Some(no_logging) = sub_exec.value_of("no_logging") {
                    AppConfig::set("no_logging", no_logging)?;
                }
            }
            if sub_exec.is_present("pyspy_blocking") {
                if let Some(pyspy_blocking) = sub_exec.value_of("pyspy_blocking") {
                    AppConfig::set("pyspy_blocking", pyspy_blocking)?;
                }
            }
            if sub_exec.is_present("rbspy_blocking") {
                if let Some(rbspy_blocking) = sub_exec.value_of("rbspy_blocking") {
                    AppConfig::set("rbspy_blocking", rbspy_blocking)?;
                }
            }
            if sub_exec.is_present("sample_rate") {
                if let Some(sample_rate) = sub_exec.value_of("sample_rate") {
                    AppConfig::set("sample_rate", sample_rate)?;
                }
            }
            if sub_exec.is_present("server_address") {
                if let Some(server_address) = sub_exec.value_of("server_address") {
                    AppConfig::set("server_address", server_address)?;
                }
            }
            if sub_exec.is_present("tag") {
                if let Some(tags) = sub_exec.values_of("tag") {
                    // Join tags by ;
                    let tag: String = tags.collect::<Vec<&str>>().join(";");

                    AppConfig::set("tag", tag.as_str())?;
                }
            }
            // TODO: placeholder for future implementation
            //if sub_exec.is_present("upstream_request_timeout") {
            //if let Some(upstream_request_timeout) =
            //sub_exec.value_of("upstream_request_timeout")
            //{
            //AppConfig::set("upstream_request_timeout", upstream_request_timeout)?;
            //}
            //}
            //if sub_exec.is_present("upstream_threads") {
            //if let Some(upstream_threads) = sub_exec.value_of("upstream_threads") {
            //AppConfig::set("upstream_threads", upstream_threads)?;
            //}
            //}
            if sub_exec.is_present("application_name") {
                if let Some(application_name) = sub_exec.value_of("application_name") {
                    AppConfig::set("application_name", application_name)?;
                }
            }
            // TODO: placeholder for future implementation
            //if sub_exec.is_present("auth_token") {
            //if let Some(auth_token) = sub_exec.value_of("auth_token") {
            //AppConfig::set("auth_token", auth_token)?;
            //}
            //}
            if sub_exec.is_present("detect_subprocesses") {
                if let Some(detect_subprocesses) = sub_exec.value_of("detect_subprocesses") {
                    AppConfig::set("detect_subprocesses", detect_subprocesses)?;
                }
            }
            if sub_exec.is_present("spy_name") {
                if let Some(spy_name) = sub_exec.value_of("spy_name") {
                    AppConfig::set("spy_name", spy_name)?;
                }
            }
            if sub_exec.is_present("user_name") {
                if let Some(user_name) = sub_exec.value_of("user_name") {
                    AppConfig::set("user_name", user_name)?;
                }
            }
            if sub_exec.is_present("group_name") {
                if let Some(group_name) = sub_exec.value_of("group_name") {
                    AppConfig::set("group_name", group_name)?;
                }
            }
        }

        Ok(())
    }

    pub fn merge_config(config_file: Option<&Path>) -> Result<()> {
        // Merge settings with config file if there is one
        if let Some(config_file_path) = config_file {
            {
                CONFIG
                    .write()?
                    .merge(config::File::with_name(config_file_path.to_str().unwrap()))?;
            }
        }
        Ok(())
    }

    // Set CONFIG
    pub fn set(key: &str, value: &str) -> Result<()> {
        {
            // Set Property
            CONFIG.write()?.set(key, value)?;
        }

        Ok(())
    }

    //pub fn setT<T>(key: &str, value: T) -> Result<()>
    //where
    //T: serde::Serialize,
    //{
    //{
    //CONFIG.write()?.set::<T>(key, value)?;
    //}
    //Ok(())
    //}

    // Get a single value
    pub fn get<'de, T>(key: &'de str) -> Result<T>
    where
        T: serde::Deserialize<'de>,
    {
        Ok(CONFIG.read()?.get::<T>(key)?)
    }

    // Get CONFIG
    // This clones Config (from RwLock<Config>) into a new AppConfig object.
    // This means you have to fetch this again if you changed the configuration.
    pub fn fetch() -> Result<AppConfig> {
        // Get a Read Lock from RwLock
        let r = CONFIG.read()?;

        // Clone the Config object
        let config_clone = r.deref().clone();

        // Coerce Config into AppConfig
        Ok(config_clone.try_into()?)
    }
}
