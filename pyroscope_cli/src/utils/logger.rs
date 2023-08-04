use slog::{o, Drain};

use super::{app_config::AppConfig, error::Result, types::LogLevel};

pub fn setup_logging() -> Result<slog_scope::GlobalLoggerGuard> {
    // Setup Logging
    let guard = slog_scope::set_global_logger(default_root_logger()?);
    let _log_guard = slog_stdlog::init()?;

    Ok(guard)
}

pub fn default_root_logger() -> Result<slog::Logger> {
    // Create drains
    let drain = slog::Duplicate(default_discard()?, default_discard()?).fuse();

    // Merge drains
    let drain = slog::Duplicate(default_term_drain().unwrap_or(default_discard()?), drain).fuse();

    // Filter drain
    let drain = LevelFilter { drain }.fuse();

    // Create Logger
    let logger = slog::Logger::root(drain, o!("who" => "pyroscope-cli"));

    // Return Logger
    Ok(logger)
}

fn default_discard() -> Result<slog_async::Async> {
    let drain = slog_async::Async::default(slog::Discard);

    Ok(drain)
}

// term drain: Log to Terminal
fn default_term_drain() -> Result<slog_async::Async> {
    let decorator = slog_term::TermDecorator::new().build();
    let term = slog_term::FullFormat::new(decorator);

    let drain = slog_async::Async::default(term.build().fuse());

    Ok(drain)
}

struct LevelFilter<D> {
    drain: D,
}

impl<D> Drain for LevelFilter<D>
where
    D: Drain,
{
    type Ok = Option<D::Ok>;
    type Err = Option<D::Err>;

    fn log(
        &self, record: &slog::Record, values: &slog::OwnedKVList,
    ) -> std::result::Result<Self::Ok, Self::Err> {
        // Discard logs if no_logging flag is set
        let logging = AppConfig::get::<bool>("no_logging").unwrap_or(false);
        if logging {
            return Ok(None);
        }

        // TODO: This is probably expensive (and should be cached)
        let log_level = AppConfig::get::<LogLevel>("log_level").unwrap_or(LogLevel::Error);

        // convert log level to slog level
        let current_level = match log_level {
            LogLevel::Trace => slog::Level::Trace,
            LogLevel::Debug => slog::Level::Debug,
            LogLevel::Info => slog::Level::Info,
            LogLevel::Warn => slog::Level::Warning,
            LogLevel::Error => slog::Level::Error,
            LogLevel::Critical => slog::Level::Critical,
        };

        // check if log level is above current level
        if record.level().is_at_least(current_level) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}
