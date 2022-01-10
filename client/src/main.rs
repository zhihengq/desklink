use anyhow::Result;
use desklink_client::config::Config;
use desklink_common::{logging, rpc::desk_service_client::DeskServiceClient};
use slog::{o, Drain, LevelFilter, Logger};
use std::sync::Mutex;

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

    // Run command
    let client = DeskServiceClient::connect(config.client.server).await?;
    desklink_client::run(client, config.command).await
}
