use anyhow::Result;
use config::Command;

pub mod config;
mod subcommands;

type Client = desklink_common::rpc::desk_service_client::DeskServiceClient<
    tonic::transport::channel::Channel,
>;

pub async fn run(client: Client, command: Command) -> Result<()> {
    match command {
        Command::Status => unimplemented!(),
        Command::Stop => unimplemented!(),
        Command::To { target, wait } => unimplemented!(),
    }
}
