use btleplug::api::BDAddr;
use clap::Parser;
use desklink_common::{deserialize_log_level, PROJECT_NAME};
use directories::ProjectDirs;
use serde::Deserialize;
use std::{ffi::OsString, io, net::SocketAddr, path::PathBuf};
use thiserror::Error;
use tracing::Level;

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

    #[error("Invalid path to the log file {0}")]
    InvalidLogfile(PathBuf),
}

mod args {
    use super::*;

    #[derive(Parser, Debug)]
    pub struct Args {
        /// Log level [trace|debug|info|warning|error|critical]
        #[clap(short = 'v', long)]
        pub log_level: Option<Level>,

        /// Log file
        #[clap(short = 'f', long)]
        pub log_file: Option<PathBuf>,

        /// Desk MAC address
        #[clap(short, long)]
        pub desk: Option<BDAddr>,

        /// Override config file path
        #[clap(short, long)]
        pub config: Option<PathBuf>,

        /// Server bind address and port
        #[clap(short, long)]
        pub server: Option<SocketAddr>,
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
        #[serde(deserialize_with = "deserialize_log_level")]
        pub level: Option<Level>,
        pub file: Option<PathBuf>,
    }

    #[derive(Deserialize)]
    pub struct ServerConfig {
        pub address: Option<SocketAddr>,
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
    pub file: Option<(PathBuf, OsString)>,
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
        let args = args::Args::parse();
        let (config_path, is_explicit) = match args.config {
            Some(path) => (Some(path), true),
            None => {
                let dirs = ProjectDirs::from("", "", PROJECT_NAME);
                let path = dirs.map(|dirs| dirs.config_dir().join("server.toml"));
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
            log: {
                let (log_level, log_file) = match toml_config.log {
                    Some(log) => (log.level, log.file),
                    None => (None, None),
                };
                LogConfig {
                    level: args.log_level.or(log_level).unwrap_or(Level::INFO),
                    file: args
                        .log_file
                        .or(log_file)
                        .map(|path| {
                            if let (Some(directory), Some(file_name)) =
                                (path.parent(), path.file_name())
                            {
                                Ok((directory.to_owned(), file_name.to_owned()))
                            } else {
                                Err(ConfigError::InvalidLogfile(path))
                            }
                        })
                        .transpose()?,
                }
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
