use anyhow::Result;
use desklink_common::{info, logging};
use desklink_server::{
    config::Config,
    controllers,
    desk::Desk,
    service::{DeskService, DeskServiceServer},
};
use futures::{FutureExt, StreamExt};
use signal_hook::consts::signal;
use signal_hook_tokio::Signals;
use slog::{o, Drain, Duplicate, Logger};
use std::{fs::OpenOptions, sync::Mutex};
use tokio::sync::watch;
use tonic::transport::Server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Config
    let config = Config::get()?;
    #[cfg(debug_assertions)]
    println!("{:#?}", config);

    // Logger
    let console_decorator = slog_term::TermDecorator::new().build();
    let console_drain = slog_term::FullFormat::new(console_decorator)
        .use_original_order()
        .build()
        .filter_level(config.log.level);
    let root = if let Some(file) = config.log.file {
        let file = OpenOptions::new().append(true).create(true).open(file)?;
        let file_drain = slog_json::Json::default(file).filter_level(config.log.level);

        Logger::root(
            Mutex::new(Duplicate::new(file_drain, console_drain)).fuse(),
            o!(),
        )
    } else {
        Logger::root(Mutex::new(console_drain).fuse(), o!())
    };
    logging::set(root);

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
