use crate::backend::{
    Backend, BackendConfig, BackendImpl, BackendUninitialized, Report, ReportBatch, ReportData,
    StackBuffer, StackFrame, StackTrace, ThreadTag,
};
use crate::error::{PyroscopeError, Result};
use blazesym::symbolize::source::Process;
use blazesym::symbolize::source::Source;
use blazesym::symbolize::Input;
use blazesym::symbolize::{Builder, Symbolizer};
use blazesym::Addr;
use blazesym::Pid;
use pprof::{ProfilerGuard, ProfilerGuardBuilder};
use std::{
    collections::HashMap,
    ffi::OsStr,
    ops::Deref,
    sync::{Arc, Mutex},
};

const LOG_TAG: &str = "Pyroscope::Pprofrs";

#[derive(Debug)]
pub struct PprofConfig {
    pub sample_rate: u32,
}

impl Default for PprofConfig {
    fn default() -> Self {
        PprofConfig { sample_rate: 100 }
    }
}

pub struct Pprof<'a> {
    buffer: Arc<Mutex<StackBuffer>>,
    config: PprofConfig,
    backend_config: BackendConfig,
    guard: ProfilerGuard<'a>,

    symbolizer: Symbolizer,
}

impl std::fmt::Debug for Pprof<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "Pprof Backend")
    }
}

impl<'a> Pprof<'a> {
    pub fn new(config: PprofConfig, backend_config: BackendConfig) -> Result<Self> {
        let symbolizer = Symbolizer::builder().build();

        let profiler = ProfilerGuardBuilder::default()
            .frequency(config.sample_rate as i32)
            .build()
            .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?;

        // *self.guard.lock()? = Some(profiler);
        //
        // Ok(())

        Ok(Pprof {
            buffer: Arc::new(Mutex::new(StackBuffer::default())),
            config,
            backend_config,
            guard: profiler,
            symbolizer,
        })
    }
}

impl Pprof<'_> {
    pub fn report(&mut self) -> Result<ReportBatch> {
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
}

impl Pprof<'_> {
    pub fn dump_report(&mut self) -> Result<()> {
        let report = self
            .guard
            .report()
            .build_unresolved_and_reset()
            .map_err(|e| PyroscopeError::new(e.to_string().as_str()))?;

        // let stack_buffer = Into::<StackBuffer>::into(Into::<StackBufferWrapper>::into((
        //     report,
        //     &self.backend_config,
        // )));
        //
        // {
        //     let mut buffer = self.buffer.lock()?;
        //     for (stacktrace, count) in stack_buffer.data {
        //         buffer.record_with_count(stacktrace, count)?;
        //     }
        // }

        Ok(())
    }
}
//
// struct StackBufferWrapper(StackBuffer);
//
// impl From<StackBufferWrapper> for StackBuffer {
//     fn from(stackbuffer: StackBufferWrapper) -> Self {
//         stackbuffer.0
//     }
// }
//
// impl From<(pprof::Report, &BackendConfig)> for StackBufferWrapper {
//     fn from(arg: (pprof::Report, &BackendConfig)) -> Self {
//         let (report, config) = arg;
//         let buffer_data: HashMap<StackTrace, usize> = report
//             .data
//             .iter()
//             .map(|(key, value)| {
//                 (
//                     Into::<StackTraceWrapper>::into((key.to_owned(), config)).into(),
//                     value.to_owned() as usize,
//                 )
//             })
//             .collect();
//         StackBufferWrapper(StackBuffer::new(buffer_data))
//     }
// }
//
// struct StackTraceWrapper(StackTrace);
//
// impl From<StackTraceWrapper> for StackTrace {
//     fn from(stack_trace: StackTraceWrapper) -> Self {
//         stack_trace.0
//     }
// }
//
// impl From<(pprof::Frames, &BackendConfig)> for StackTraceWrapper {
//     fn from(arg: (pprof::Frames, &BackendConfig)) -> Self {
//         let (frames, config) = arg;
//         let thread_id = frames.thread_id as libc::pthread_t;
//         StackTraceWrapper(StackTrace::new(
//             config,
//             None,
//             Some(thread_id.into()),
//             Some(frames.thread_name),
//             frames
//                 .frames
//                 .concat()
//                 .iter()
//                 .map(|frame| Into::<StackFrameWrapper>::into(frame.to_owned()).into())
//                 .collect(),
//         ))
//     }
// }
//
// struct StackFrameWrapper(StackFrame);
//
// impl From<StackFrameWrapper> for StackFrame {
//     fn from(stack_frame: StackFrameWrapper) -> Self {
//         stack_frame.0
//     }
// }
//
// impl From<pprof::Symbol> for StackFrameWrapper {
//     fn from(symbol: pprof::Symbol) -> Self {
//         StackFrameWrapper(StackFrame::new(
//             None,
//             Some(symbol.name()),
//             Some(
//                 symbol
//                     .filename
//                     .clone()
//                     .unwrap_or_default()
//                     .file_name()
//                     .unwrap_or_else(|| OsStr::new(""))
//                     .to_str()
//                     .unwrap_or("")
//                     .to_string(),
//             ),
//             Some(
//                 symbol
//                     .filename
//                     .unwrap_or_default()
//                     .to_str()
//                     .unwrap_or("")
//                     .to_string(),
//             ),
//             None,
//             symbol.lineno,
//         ))
//     }
// }
