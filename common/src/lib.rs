use serde::{de::Deserializer, Deserialize};
use std::str::FromStr;
use tracing::Level;

pub const PROJECT_NAME: &str = "desklink";

pub mod rpc {
    tonic::include_proto!("desk_service");
}

pub fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Option<Level>, D::Error>
where
    D: Deserializer<'de>,
{
    let name: Option<String> = Option::deserialize(deserializer)?;
    name.map(|name| {
        Level::from_str(&name)
            .map_err(|_| serde::de::Error::custom(format!("Invalid log level: '{}'", name)))
    })
    .transpose()
}
