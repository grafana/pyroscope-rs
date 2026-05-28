// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use spin::RwLock;

use crate::backend::pprofrs::frames::{Frames, UnresolvedFrames};
use crate::backend::pprofrs::profiler::Profiler;
use crate::backend::pprofrs::timer::ReportTiming;

use crate::backend::pprofrs::{Error, Result};

/// The final presentation of a report which is actually an `HashMap` from `Frames` to isize (count).
pub struct Report {
    /// Key is a backtrace captured by profiler and value is count of it.
    pub data: HashMap<Frames, isize>,

    /// Collection frequency, start time, duration.
    pub timing: ReportTiming,
}

/// The presentation of an unsymbolicated report which is actually an `HashMap` from `UnresolvedFrames` to isize (count).
pub struct UnresolvedReport {
    /// key is a backtrace captured by profiler and value is count of it.
    pub data: HashMap<UnresolvedFrames, isize>,

    /// Collection frequency, start time, duration.
    pub timing: ReportTiming,
}

type FramesPostProcessor = Box<dyn Fn(&mut Frames)>;

/// A builder of `Report` and `UnresolvedReport`. It builds report from a running `Profiler`.
pub struct ReportBuilder<'a> {
    frames_post_processor: Option<FramesPostProcessor>,
    profiler: &'a RwLock<Result<Profiler>>,
    timing: ReportTiming,
}

impl<'a> ReportBuilder<'a> {
    pub(crate) fn new(profiler: &'a RwLock<Result<Profiler>>, timing: ReportTiming) -> Self {
        Self {
            frames_post_processor: None,
            profiler,
            timing,
        }
    }

    /// Set `frames_post_processor` of a `ReportBuilder`. Before finally building a report, `frames_post_processor`
    /// will be applied to every Frames.
    pub fn frames_post_processor<T>(&mut self, frames_post_processor: T) -> &mut Self
    where
        T: Fn(&mut Frames) + 'static,
    {
        self.frames_post_processor
            .replace(Box::new(frames_post_processor));

        self
    }

    /// Build an `UnresolvedReport`
    pub fn build_unresolved(&self) -> Result<UnresolvedReport> {
        let mut hash_map = HashMap::new();

        match self.profiler.read().as_ref() {
            Err(err) => {
                log::error!("Error in creating profiler: {}", err);
                Err(Error::CreatingError)
            }
            Ok(profiler) => {
                profiler.data.try_iter()?.for_each(|entry| {
                    let count = entry.count;
                    if count > 0 {
                        let key = &entry.item;
                        match hash_map.get_mut(key) {
                            Some(value) => {
                                *value += count;
                            }
                            None => {
                                match hash_map.insert(key.clone(), count) {
                                    None => {}
                                    Some(_) => {
                                        unreachable!();
                                    }
                                };
                            }
                        }
                    }
                });

                Ok(UnresolvedReport {
                    data: hash_map,
                    timing: self.timing.clone(),
                })
            }
        }
    }

    /// Build a `Report`.
    pub fn build(&self) -> Result<Report> {
        self.build_and_clear(false)
    }

    /// Build a `Report`. If `clear` is true, atomically clears the
    /// profiler's sample data under the same write lock.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CreatingError`] if the profiler lock is poisoned.
    /// Returns an I/O error if reading the overflow backing file or clearing the collector fails.
    ///
    /// NOTE: pyroscope patch — added to support periodic report collection
    /// without recreating the ProfilerGuard. See https://github.com/grafana/pyroscope-rs/issues/399
    pub fn build_and_clear(&self, clear: bool) -> Result<Report> {
        let mut hash_map = HashMap::new();

        match self.profiler.write().as_mut() {
            Err(err) => {
                log::error!("Error in creating profiler: {}", err);
                Err(Error::CreatingError)
            }
            Ok(profiler) => {
                profiler.data.try_iter()?.for_each(|entry| {
                    let count = entry.count;
                    if count > 0 {
                        let mut key = Frames::from(entry.item.clone());
                        if let Some(processor) = &self.frames_post_processor {
                            processor(&mut key);
                        }

                        match hash_map.get_mut(&key) {
                            Some(value) => {
                                *value += count;
                            }
                            None => {
                                match hash_map.insert(key, count) {
                                    None => {}
                                    Some(_) => {
                                        unreachable!();
                                    }
                                };
                            }
                        }
                    }
                });

                if clear {
                    profiler.clear()?;
                }

                Ok(Report {
                    data: hash_map,
                    timing: self.timing.clone(),
                })
            }
        }
    }
}

/// This will generate Report in a human-readable format:
///
/// ```shell
/// FRAME: pprof::profiler::perf_signal_handler::h7b995c4ab2e66493 -> FRAME: Unknown -> FRAME: {func1} ->
/// FRAME: {func2} -> FRAME: {func3} ->  THREAD: {thread_name} {count}
/// ```
impl Debug for Report {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for (key, val) in self.data.iter() {
            write!(f, "{key:?} {val}")?;
            writeln!(f)?;
        }

        Ok(())
    }
}
