use crate::backend::pprofrs::{ProfilerGuard, ProfilerGuardBuilder};
use crate::backend::{
    Backend, BackendConfig, BackendImpl, BackendUninitialized, Report, ReportBatch, ReportData,
    StackBuffer, StackFrame, StackTrace, ThreadTag, ThreadTagsSet,
};
use crate::error::{PyroscopeError, Result};
use std::{
    collections::HashMap,
    ffi::OsStr,
    ops::Deref,
    sync::{Arc, Mutex},
};

const LOG_TAG: &str = "Pyroscope::Pprofrs";

pub fn pprof_backend(
    config: PprofConfig,
    backend_config: BackendConfig,
) -> BackendImpl<BackendUninitialized> {
    BackendImpl::new(Box::new(Pprof::new(config, backend_config)))
}

#[derive(Debug)]
pub struct PprofConfig {
    pub sample_rate: u32,
    /// Preserve timestamped observations with native OS thread IDs (Linux/macOS).
    pub individual_samples: bool,
    /// Maximum samples per collection window, including windows split by tag changes.
    /// None is unlimited; zero disables collection.
    pub sample_limit: Option<usize>,
}

impl Default for PprofConfig {
    fn default() -> Self {
        PprofConfig {
            sample_rate: 100,
            individual_samples: false,
            sample_limit: None,
        }
    }
}

impl PprofConfig {
    /// Retain each observation for timeline analysis. Disabled by default.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn retain_individual_samples(mut self) -> Self {
        self.individual_samples = true;
        self
    }

    /// Bound collection until the next report or thread-tag change resets the sampler.
    pub fn sample_limit(mut self, limit: usize) -> Self {
        self.sample_limit = Some(limit);
        self
    }
}

#[derive(Default)]
pub struct Pprof<'a> {
    buffer: Arc<Mutex<StackBuffer>>,
    config: PprofConfig,
    backend_config: BackendConfig,
    guard: Arc<Mutex<Option<ProfilerGuard<'a>>>>,
    ruleset: ThreadTagsSet,
}

impl std::fmt::Debug for Pprof<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "Pprof Backend")
    }
}

impl<'a> Pprof<'a> {
    pub fn new(config: PprofConfig, backend_config: BackendConfig) -> Self {
        Pprof {
            buffer: Arc::new(Mutex::new(StackBuffer::default())),
            config,
            backend_config,
            guard: Arc::new(Mutex::new(None)),
            ruleset: ThreadTagsSet::default(),
        }
    }
}

impl Backend for Pprof<'_> {
    fn shutdown(self: Box<Self>) -> Result<()> {
        log::trace!(target: LOG_TAG, "Shutting down sampler thread");
        Ok(())
    }

    fn initialize(&mut self) -> Result<()> {
        let mut profiler =
            ProfilerGuardBuilder::default().frequency(self.config.sample_rate as i32);
        if self.config.individual_samples {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                profiler = profiler.retain_individual_samples();
            }
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            return Err(PyroscopeError::new(
                "individual samples require Linux or macOS",
            ));
        }
        if let Some(limit) = self.config.sample_limit {
            profiler = profiler.sample_limit(limit);
        }

        let profiler = profiler
            .build()
            .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?;

        *self.guard.lock()? = Some(profiler);

        Ok(())
    }

    fn report(&mut self) -> Result<ReportBatch> {
        self.dump_report()?;

        let buffer = self.buffer.clone();

        let report: StackBuffer = buffer.lock()?.deref().to_owned();

        let reports: Vec<Report> = report.into();

        buffer.lock()?.clear();

        Ok(ReportBatch {
            profile_type: "process_cpu".into(),
            data: ReportData::Reports(reports),
        })
    }

    fn add_tag(&self, tag: ThreadTag) -> Result<()> {
        if self.guard.lock()?.as_ref().is_some() {
            self.dump_report()?;
        }

        self.ruleset.add(tag)?;

        Ok(())
    }

    fn remove_tag(&self, tag: ThreadTag) -> Result<()> {
        if self.guard.lock()?.as_ref().is_some() {
            self.dump_report()?;
        }

        self.ruleset.remove(tag)?;

        Ok(())
    }
}

impl Pprof<'_> {
    /// Workaround for pprof-rs to interrupt the profiler.
    pub fn dump_report(&self) -> Result<()> {
        let report = self
            .guard
            .lock()?
            .as_ref()
            .ok_or_else(|| PyroscopeError::new("pprof-rs: ProfilerGuard report error"))?
            .report()
            .build_and_clear(true)
            .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?;

        let stack_buffer = Into::<StackBuffer>::into(Into::<StackBufferWrapper>::into((
            report,
            &self.backend_config,
        )));

        let data: HashMap<StackTrace, usize> = stack_buffer
            .data
            .iter()
            .map(|(stacktrace, ss)| {
                let stacktrace = stacktrace.to_owned().add_tag_rules(&self.ruleset);
                (stacktrace, ss.to_owned())
            })
            .collect();

        let buffer = self.buffer.clone();

        for (stacktrace, count) in data {
            buffer.lock()?.record_with_count(stacktrace, count)?;
        }

        Ok(())
    }
}

struct StackBufferWrapper(StackBuffer);

impl From<StackBufferWrapper> for StackBuffer {
    fn from(stackbuffer: StackBufferWrapper) -> Self {
        stackbuffer.0
    }
}

impl From<(crate::backend::pprofrs::Report, &BackendConfig)> for StackBufferWrapper {
    fn from(arg: (crate::backend::pprofrs::Report, &BackendConfig)) -> Self {
        let (report, config) = arg;
        let buffer_data: HashMap<StackTrace, usize> = report
            .data
            .iter()
            .map(|(key, value)| {
                (
                    Into::<StackTraceWrapper>::into((key.to_owned(), config)).into(),
                    value.to_owned() as usize,
                )
            })
            .collect();
        StackBufferWrapper(StackBuffer::new(buffer_data))
    }
}

struct StackTraceWrapper(StackTrace);

impl From<StackTraceWrapper> for StackTrace {
    fn from(stack_trace: StackTraceWrapper) -> Self {
        stack_trace.0
    }
}

impl From<(crate::backend::pprofrs::Frames, &BackendConfig)> for StackTraceWrapper {
    fn from(arg: (crate::backend::pprofrs::Frames, &BackendConfig)) -> Self {
        let (frames, config) = arg;
        let thread_id = frames.thread_id as libc::pthread_t;
        let mut stack = StackTrace::new(
            config,
            None,
            Some(thread_id.into()),
            Some(frames.thread_name),
            frames
                .frames
                .concat()
                .iter()
                .map(|frame| Into::<StackFrameWrapper>::into(frame.to_owned()).into())
                .collect(),
        );
        if frames.trace_timestamp_ns != 0 {
            stack.observation = Some(crate::backend::types::Observation {
                trace_timestamp_ns: frames.trace_timestamp_ns,
                os_thread_id: frames.os_thread_id,
                sample_timestamp: frames.sample_timestamp,
            });
        }
        StackTraceWrapper(stack)
    }
}

struct StackFrameWrapper(StackFrame);

impl From<StackFrameWrapper> for StackFrame {
    fn from(stack_frame: StackFrameWrapper) -> Self {
        stack_frame.0
    }
}

impl From<crate::backend::pprofrs::Symbol> for StackFrameWrapper {
    fn from(symbol: crate::backend::pprofrs::Symbol) -> Self {
        StackFrameWrapper(StackFrame::new(
            None,
            Some(symbol.name()),
            Some(
                symbol
                    .filename
                    .clone()
                    .unwrap_or_default()
                    .file_name()
                    .unwrap_or_else(|| OsStr::new(""))
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
            ),
            Some(
                symbol
                    .filename
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("")
                    .to_string(),
            ),
            None,
            symbol.lineno,
        ))
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use crate::backend::Tag;
    use std::time::{Duration, SystemTime};

    #[test]
    fn observations_survive_conversion_tagging_grouping_and_encoding() {
        for individual in [false, true] {
            let config = BackendConfig::default();
            let rules = ThreadTagsSet::default();
            let tid = (7 as libc::pthread_t).into();
            rules
                .add(ThreadTag::new(tid, Tag::new("work".into(), "test".into())))
                .unwrap();
            let mut buffer = StackBuffer::default();
            for offset in 1..=2 {
                let frames = crate::backend::pprofrs::Frames {
                    frames: vec![],
                    thread_name: "worker".into(),
                    thread_id: 7,
                    sample_timestamp: SystemTime::UNIX_EPOCH + Duration::from_nanos(offset),
                    trace_timestamp_ns: if individual { offset } else { 0 },
                    os_thread_id: if individual { 42 } else { 0 },
                };
                let stack: StackTrace = StackTraceWrapper::from((frames, &config)).into();
                buffer.record(stack.add_tag_rules(&rules)).unwrap();
            }
            let reports: Vec<Report> = buffer.into();
            assert_eq!(reports.len(), 1, "observations must share a profile series");
            assert!(reports[0]
                .metadata
                .tags
                .contains(&Tag::new("work".into(), "test".into())));
            let profile = crate::encode::pprof::encode(&reports, 100, 0, 1_000_000_000);
            assert_eq!(profile.sample.len(), if individual { 2 } else { 1 });
            assert_eq!(
                profile.sample.iter().map(|s| s.value[0]).sum::<i64>(),
                20_000_000
            );
            let mut timestamps = vec![];
            for sample in &profile.sample {
                let numeric: HashMap<_, _> = sample
                    .label
                    .iter()
                    .filter(|label| label.str == 0)
                    .map(|label| (profile.string_table[label.key as usize].as_str(), label.num))
                    .collect();
                if individual {
                    assert_eq!(sample.value, vec![10_000_000]);
                    assert_eq!(numeric["os_thread_id"], 42);
                    timestamps.push(numeric["trace_timestamp_ns"]);
                } else {
                    assert!(numeric.is_empty());
                }
            }
            if individual {
                timestamps.sort_unstable();
                assert_eq!(timestamps, vec![1, 2]);
            }
        }
    }
}
