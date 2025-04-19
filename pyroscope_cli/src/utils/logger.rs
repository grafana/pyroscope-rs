
use super::{
    error::Result,
    types::LogLevel,
};

pub fn setup_logging(log_level: LogLevel) -> Result<()> {
    let log_level = match log_level {
        LogLevel::Trace => {log::LevelFilter::Trace}
        LogLevel::Debug => {log::LevelFilter::Debug}
        LogLevel::Info => {log::LevelFilter::Info}
        LogLevel::Warn => {log::LevelFilter::Warn}
        LogLevel::Error => {log::LevelFilter::Error}
    };
    env_logger::builder()
        .filter_level(log_level)
        .try_init()?;
    Ok(())
}
