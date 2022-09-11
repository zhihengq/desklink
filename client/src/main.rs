use anyhow::Result;
use desklink_client::config::Config;
use desklink_common::rpc::desk_service_client::DeskServiceClient;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Config
    let config = Config::get()?;
    #[cfg(debug_assertions)]
    println!("{:#?}", config);

    // Logger
    tracing_subscriber::fmt()
        .with_max_level(config.log.level)
        .init();

    // Run command
    let client = DeskServiceClient::connect(config.client.server).await?;
    desklink_client::run(client, config.command).await
}
