use clap::Parser;
use desklink_common::{deserialize_log_level, PROJECT_NAME};
use directories::ProjectDirs;
use serde::{de::Deserializer, Deserialize};
use std::{collections::HashMap, io, path::PathBuf, str::FromStr};
use thiserror::Error;
use tonic::transport::Endpoint;
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

    #[error("Preset `{0}` not found")]
    PresetNotFound(String),
}

mod args {
    use super::*;

    #[derive(Parser, Debug)]
    pub struct Args {
        /// Log level [trace|debug|info|warning|error|critical]
        #[clap(short = 'v', long)]
        pub log_level: Option<Level>,

        /// Override config file path
        #[clap(short, long)]
        pub config: Option<PathBuf>,

        /// Server address and port
        #[clap(short, long)]
        pub server: Option<Endpoint>,

        /// Command
        #[clap(subcommand)]
        pub command: Command,
    }

    #[derive(Parser, Debug)]
    pub enum Command {
        /// Check the current position and velocity of the desk
        Status,

        /// Stop desk motion and cancel in-progress commands
        Stop,

        /// Move desk to target position
        To {
            /// Target position in cm, or a preset name
            target: String,

            /// Monitor desk position and wait until target is reached
            #[clap(short, long)]
            wait: bool,
        },
    }
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    pub struct Config {
        pub log: Option<LogConfig>,
        pub client: Option<ClientConfig>,
        pub presets: HashMap<String, f32>,
    }

    #[derive(Deserialize)]
    pub struct LogConfig {
        #[serde(deserialize_with = "deserialize_log_level")]
        pub level: Option<Level>,
    }

    #[derive(Deserialize)]
    pub struct ClientConfig {
        #[serde(deserialize_with = "deserialize_endpoint")]
        pub server: Option<Endpoint>,
    }

    pub fn deserialize_endpoint<'de, D>(deserializer: D) -> Result<Option<Endpoint>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name: Option<String> = Option::deserialize(deserializer)?;
        name.map(|name| Endpoint::from_str(&name).map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[derive(Debug)]
pub struct Config {
    pub log: LogConfig,
    pub client: ClientConfig,
    pub command: Command,
}

#[derive(Debug)]
pub struct LogConfig {
    pub level: Level,
}

#[derive(Debug)]
pub struct ClientConfig {
    pub server: Endpoint,
}

#[derive(Debug)]
pub enum Command {
    Status,
    Stop,
    To { target: f32, wait: bool },
}

impl Config {
    pub fn get() -> Result<Self, ConfigError> {
        let args = args::Args::parse();
        let (config_path, is_explicit) = match args.config {
            Some(path) => (Some(path), true),
            None => {
                let dirs = ProjectDirs::from("", "", PROJECT_NAME);
                let path = dirs.map(|dirs| dirs.config_dir().join("client.toml"));
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
                    .unwrap_or(Level::INFO),
            },
            client: ClientConfig {
                server: args
                    .server
                    .or_else(|| toml_config.client.and_then(|c| c.server))
                    .ok_or(ConfigError::MissingConfigField("server address"))?,
            },
            command: match args.command {
                args::Command::Status => Command::Status,
                args::Command::Stop => Command::Stop,
                args::Command::To { target, wait } => Command::To {
                    target: {
                        target.parse::<f32>().or_else(|_| {
                            toml_config
                                .presets
                                .get(&target)
                                .copied()
                                .ok_or(ConfigError::PresetNotFound(target))
                        })?
                    },
                    wait,
                },
            },
        };
        Ok(config)
    }
}
