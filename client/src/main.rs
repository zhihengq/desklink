use anyhow::Result;
use config::Config;
use desklink_common::{
    info, logging,
    rpc::{desk_service_client::DeskServiceClient, SubscribeStateRequest},
};
use slog::{o, Drain, LevelFilter, Logger};
use std::sync::Mutex;

mod config;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Config
    let config = Config::get()?;
    #[cfg(debug_assertions)]
    println!("{:#?}", config);

    // Logger
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build();
    let drain = LevelFilter::new(drain, config.log.level);
    let root = Logger::root(Mutex::new(drain).fuse(), o!());
    logging::set(root);

    // RPC client
    let mut client = DeskServiceClient::connect(config.client.server).await?;
    let request = SubscribeStateRequest {};
    let mut states = client.subscribe_state(request).await?.into_inner();
    while let Some(state) = states.message().await? {
        info!("{:?}", state);
    }

    Ok(())
}
