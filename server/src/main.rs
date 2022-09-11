use anyhow::Result;
use desklink_server::{
    config::Config,
    controllers,
    desk::Desk,
    service::{DeskService, DeskServiceServer},
};
use futures::{FutureExt, StreamExt};
use signal_hook::consts::signal;
use signal_hook_tokio::Signals;
use tokio::sync::watch;
use tonic::transport::Server;
use tracing::info;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Config
    let config = Config::get()?;
    #[cfg(debug_assertions)]
    println!("{:#?}", config);

    // Logger
    let subscriber_builder = tracing_subscriber::fmt().with_max_level(config.log.level);
    let mut _log_guard;
    if let Some((directory, file_name)) = config.log.file {
        let file_appender = tracing_appender::rolling::never(directory, file_name);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        subscriber_builder.with_writer(non_blocking).json().init();
        _log_guard = guard;
    } else {
        let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
        subscriber_builder.with_writer(non_blocking).init();
        _log_guard = guard;
    }

    // Desk controller driver
    let desk = Desk::find(config.desk.address).await?;
    let mut controller = controllers::create_controller(desk);
    let (tx, rx) = watch::channel(Default::default());
    let join_controller = tokio::spawn(async move {
        controller
            .drive(rx)
            .await
            .unwrap_or_else(|e| panic!("{}", e))
    });

    // Shutdown signal
    let mut signals = Signals::new([signal::SIGINT, signal::SIGTERM])?;
    let shutdown = signals.next().map(|sig| {
        info!(
            "{} received, finishing existing client connections",
            match sig.unwrap() {
                signal::SIGINT => "SIGINT",
                signal::SIGTERM => "SIGTERM",
                _ => unreachable!(),
            }
        );
    });

    // RPC server
    info!("Starting server...");
    let svc = DeskServiceServer::new(DeskService::new(tx));
    Server::builder()
        .add_service(svc)
        .serve_with_shutdown(config.server.address, shutdown)
        .await?;
    info!("Shutting down server...");

    join_controller.await?;
    Ok(())
}
