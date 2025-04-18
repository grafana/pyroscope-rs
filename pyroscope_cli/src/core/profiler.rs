use crate::utils::{
    error::{Error, Result},
    types::Spy,
};
use pyroscope::pyroscope::ReportEncoding;
use pyroscope::{pyroscope::PyroscopeAgentRunning, PyroscopeAgent};
use pyroscope_pyspy::{pyspy_backend, PyspyConfig};
use pyroscope_rbspy::{rbspy_backend, RbspyConfig};
use std::collections::HashMap;
use std::vec;
use crate::cli::{ValidatedArgs};

#[derive(Debug, Default)]
pub struct Profiler {
    agent: Option<PyroscopeAgent<PyroscopeAgentRunning>>,
}

impl Profiler {
    pub fn init(&mut self, pid: i32, args: &ValidatedArgs) -> Result<()> {
        let server_address: String = // todo do we still need the adhoc mode?
            if let Ok(adhoc_server_address) = std::env::var("PYROSCOPE_ADHOC_SERVER_ADDRESS") {
                adhoc_server_address
            } else {
                args.server_address.to_string()
            };

        let mut builder = PyroscopeAgent::default_builder()
            .url(server_address)
            .application_name(&args.application_name)
            .report_encoding(ReportEncoding::PPROF)
            .http_headers(parse_headers(&args.http_header)?)
            .tags(parse_tags(&args.tag)?);
        if let Some(tenant_id) = &args.tenant_id {
            builder = builder.tenant_id(tenant_id.clone());
        }
        if let Some(auth_token) = &args.auth_token {
            builder = builder.auth_token(auth_token);
        } else if let (Some(basic_auth_username), Some(basic_auth_password)) =
            (&args.basic_auth_username, &args.basic_auth_password)
        {
            builder = builder.basic_auth(basic_auth_username, basic_auth_password);
        }

        let agent = match args.spy_name {
            Spy::Pyspy => {
                let config = PyspyConfig::new(pid)
                    .sample_rate(args.sample_rate)
                    .lock_process(args.blocking)
                    .detect_subprocesses(args.detect_subprocesses)
                    .oncpu(args.oncpu)
                    .gil_only(args.pyspy_gil)
                    .native(false);

                if args.blocking {
                    log::warn!("blocking is not recommended for production use");
                }

                let backend = pyspy_backend(config);
                builder.backend(backend).build()?
            }
            Spy::Rbspy => {
                let config = RbspyConfig::new(pid)
                    .sample_rate(args.sample_rate)
                    .lock_process(args.blocking)
                    .oncpu(args.oncpu)
                    .detect_subprocesses(args.detect_subprocesses);
                let backend = rbspy_backend(config);
                builder.backend(backend).build()?
            }
        };

        let agent_running = agent.start()?;

        self.agent = Some(agent_running);

        Ok(())
    }

    pub fn stop(self) -> Result<()> {
        if let Some(agent_running) = self.agent {
            let agent_ready = agent_running.stop()?;
            agent_ready.shutdown();
        }

        Ok(())
    }
}

fn parse_headers(headers: &Option<Vec<String>>) -> Result<HashMap<String, String>> {

    let headers = parse_tags(headers)?;
    let mut res = HashMap::new();
    for (k, v) in headers {
        res.insert(k.to_string(), v.to_string());
    }
    Ok(res)
}

fn parse_tag(tag: &str) -> Result<(&str, &str)> {
    let mut tag_array = tag.split('=');
    let key = tag_array
        .next()
        .ok_or_else(|| Error::new("failed to parse tag key"))?;
    let value = tag_array
        .next()
        .ok_or_else(|| Error::new("failed to parse tag value"))?;
    Ok((key, value))
}

fn parse_tags(tags: &Option<Vec<String>>) -> Result<Vec<(&str, &str)>> {
    match tags {
        None => {Ok(vec![])}
        Some(tags) => {
            let mut res = Vec::with_capacity(tags.len());
            for tag in tags {
                res.push(parse_tag(tag)?)
            }
            Ok(res)
        }
    }
}
