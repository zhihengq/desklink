use anyhow::Result;
use desk_common::{
    info, logging,
    rpc::{desk_service_client::DeskServiceClient, SubscribeStateRequest},
};
use slog::{o, Drain, LevelFilter, Logger};
use std::sync::Mutex;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Config
    // let config = Config::get()?;
    // println!("{:#?}", config);

    // Logger
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build();
    // let drain = LevelFilter::new(drain, config.log.level);
    let root = Logger::root(Mutex::new(drain).fuse(), o!());
    logging::set(root);

    // RPC client
    let mut client = DeskServiceClient::connect("http://[::1]:3375").await?;
    let request = SubscribeStateRequest {};
    let mut states = client.subscribe_state(request).await?.into_inner();
    while let Some(state) = states.message().await? {
        info!("{:?}", state);
    }

    Ok(())
}
