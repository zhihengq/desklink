use btleplug::api::BDAddr;
use directories::ProjectDirs;
use serde::{de::Deserializer, Deserialize};
use slog::Level;
use std::{io, net::SocketAddr, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: `{path}`")]
    IoError {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("TOML parsing error")]
    TomlError(#[from] toml::de::Error),

    #[error("Missing config field for {0}")]
    MissingConfigField(&'static str),
}

#[derive(Error, Debug)]
pub enum LogLevelError {
    #[error("Invalid log level: `{0}`")]
    InvalidLogLevel(String),
}

mod args {
    use super::*;

    #[derive(StructOpt)]
    pub struct Args {
        /// Log level [trace|debug|info|warning|error|critical]
        #[structopt(short = "v", long, parse(try_from_str = parse_log_level))]
        pub log_level: Option<Level>,

        /// Desk MAC address
        #[structopt(short, long)]
        pub desk: Option<BDAddr>,

        /// Override config file path
        #[structopt(short, long, parse(from_os_str))]
        pub config: Option<PathBuf>,

        /// Server bind address and port
        #[structopt(short, long)]
        pub server: Option<SocketAddr>,
    }

    fn parse_log_level(name: &str) -> Result<Level, LogLevelError> {
        Level::from_str(name).map_err(|()| LogLevelError::InvalidLogLevel(name.to_owned()))
    }
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    pub struct Config {
        pub desk: Option<DeskConfig>,
        pub log: Option<LogConfig>,
        pub server: Option<ServerConfig>,
    }

    #[derive(Deserialize)]
    pub struct DeskConfig {
        pub address: Option<BDAddr>,
    }

    #[derive(Deserialize)]
    pub struct LogConfig {
        #[serde(default)]
        #[serde(deserialize_with = "deserialize_log_level")]
        pub level: Option<Level>,
    }

    #[derive(Deserialize)]
    pub struct ServerConfig {
        pub address: Option<SocketAddr>,
    }

    fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Option<Level>, D::Error>
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
}

#[derive(Debug)]
pub struct Config {
    pub log: LogConfig,
    pub desk: DeskConfig,
    pub server: ServerConfig,
}

#[derive(Debug)]
pub struct LogConfig {
    pub level: Level,
}

#[derive(Debug)]
pub struct DeskConfig {
    pub address: BDAddr,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub address: SocketAddr,
}

impl Config {
    pub fn get() -> Result<Self, ConfigError> {
        let args = args::Args::from_args();
        let (config_path, is_explicit) = match args.config {
            Some(path) => (Some(path), true),
            None => {
                let dirs = ProjectDirs::from("", "", "idasen-desk-controller");
                let path = dirs.map(|dirs| dirs.config_dir().join("config.toml"));
                (path, false)
            }
        };
        let config_content = match config_path {
            Some(config_path) => match std::fs::read_to_string(&config_path) {
                Ok(config) => config,
                Err(err) => {
                    if err.kind() == io::ErrorKind::NotFound && !is_explicit {
                        "".to_owned()
                    } else {
                        return Err(ConfigError::IoError {
                            path: config_path,
                            error: err,
                        });
                    }
                }
            },
            None => "".to_owned(),
        };
        let toml_config: file::Config = toml::from_str(&config_content)?;

        let config = Config {
            log: LogConfig {
                level: args
                    .log_level
                    .or_else(|| toml_config.log.and_then(|l| l.level))
                    .unwrap_or(Level::Info),
            },
            desk: DeskConfig {
                address: args
                    .desk
                    .or_else(|| toml_config.desk.and_then(|d| d.address))
                    .ok_or(ConfigError::MissingConfigField("desk MAC address"))?,
            },
            server: ServerConfig {
                address: args
                    .server
                    .or_else(|| toml_config.server.and_then(|s| s.address))
                    .ok_or(ConfigError::MissingConfigField("server bind address"))?,
            },
        };
        Ok(config)
    }
}
