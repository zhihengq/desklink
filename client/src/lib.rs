use anyhow::Result;
use config::Command;
use subcommands::{status, stop, to};

pub mod config;
mod subcommands;

type Client = desklink_common::rpc::desk_service_client::DeskServiceClient<
    tonic::transport::channel::Channel,
>;

pub async fn run(client: Client, command: Command) -> Result<()> {
    match command {
        Command::Status => status::run(client).await?,
        Command::Stop => stop::run(client).await?,
        Command::To { target, wait } => to::run(client, target, wait).await?,
    }
    Ok(())
}

trait Position {
    fn cm(self) -> String;
}

impl Position for f32 {
    fn cm(self) -> String {
        format!("{:>6.2} cm", self)
    }
}

trait Velocity {
    fn cm_per_s(self) -> String;
}

impl Velocity for f32 {
    fn cm_per_s(self) -> String {
        format!("{:>6.3} cm/s", self)
    }
}
