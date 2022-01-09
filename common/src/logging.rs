use once_cell::sync::OnceCell;
use serde::{de::Deserializer, Deserialize};
use slog::{Level, Logger};
use std::str::FromStr;
use thiserror::Error;

static LOG: OnceCell<Logger> = OnceCell::new();

pub fn set(logger: Logger) {
    LOG.set(logger).expect("logger is already initialized");
}

pub fn get() -> &'static Logger {
    LOG.get().expect("logger is not initialized")
}

#[derive(Error, Debug)]
pub enum LogLevelError {
    #[error("Invalid log level: `{0}`")]
    InvalidLogLevel(String),
}

pub fn parse_log_level(name: &str) -> Result<Level, LogLevelError> {
    Level::from_str(name).map_err(|()| LogLevelError::InvalidLogLevel(name.to_owned()))
}

pub fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Option<Level>, D::Error>
where
    D: Deserializer<'de>,
{
    let name: Option<String> = Option::deserialize(deserializer)?;
    name.map(|name| {
        Level::from_str(&name)
            .map_err(|()| serde::de::Error::custom(format!("Invalid log level: '{}'", name)))
    })
    .transpose()
}

macro_rules! create_macro {
    ($dollar:tt $logf:tt) => {
        #[macro_export]
        #[allow(unused)]
        macro_rules! $logf {
            ($dollar format:literal $dollar($dollar additional:tt)*) => {
                slog::$logf!(desklink_common::logging::get(), concat!("[{}:{}] ", $dollar format), file!(), line!()
                    $dollar($dollar additional)*)
            };
        }
    };
}

pub(crate) use create_macro;
