use crate::EnvConfig;
use chrono::Local;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

pub fn setup_logging(config: &EnvConfig) -> Result<(), log::SetLoggerError> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta)
        .warn(Color::Yellow)
        .error(Color::Red);

    let dispatch = Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "[{}] {: >5} [{}] {}",
                Local::now().format("%y/%m/%d %H:%M:%S%.3f"),
                colors.color(record.level()),
                record.target(),
                msg
            ))
        })
        .level(config.log_level.into())
        .chain(std::io::stdout());

    dispatch.apply()?;
    Ok(())
}
