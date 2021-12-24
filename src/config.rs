use anyhow::{anyhow, Result};
use btleplug::api::BDAddr;
use serde::{de::Deserializer, Deserialize};
use shellexpand::tilde;
use slog::Level;
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;

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

        /// Config file path [default: ~/.deskconfig]
        #[structopt(short, long, parse(from_os_str))]
        pub config: Option<PathBuf>,
    }

    fn parse_log_level(name: &str) -> Result<Level> {
        Level::from_str(name).map_err(|()| anyhow!(format!("Invalid log level: '{}'", name)))
    }
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    pub struct Config {
        pub desk: Option<DeskConfig>,
        pub log: Option<LogConfig>,
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
}

#[derive(Debug)]
pub struct LogConfig {
    pub level: Level,
}

#[derive(Debug)]
pub struct DeskConfig {
    pub address: BDAddr,
}

impl Config {
    pub fn get() -> Result<Self> {
        let args = args::Args::from_args();
        let file: file::Config = toml::from_str(
            &std::fs::read_to_string(
                args.config
                    .unwrap_or(PathBuf::from(tilde("~/.deskconfig").as_ref())),
            )
            .unwrap_or("".to_owned()),
        )?;

        let config = Config {
            log: LogConfig {
                level: args
                    .log_level
                    .or(file.log.and_then(|l| l.level))
                    .unwrap_or(Level::Info),
            },
            desk: DeskConfig {
                address: args
                    .desk
                    .or(file.desk.and_then(|d| d.address))
                    .expect("No desk MAC address is provided"),
            },
        };
        Ok(config)
    }
}
