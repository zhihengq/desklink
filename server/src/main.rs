use anyhow::Result;
use desk_common::{info, logging};
use desk_server::{
    config::Config,
    controllers,
    desk::Desk,
    service::{DeskService, DeskServiceServer},
};
use futures::{FutureExt, StreamExt};
use signal_hook::consts::signal;
use signal_hook_tokio::Signals;
use slog::{o, Drain, LevelFilter, Logger};
use std::sync::Mutex;
use tokio::sync::watch;
use tonic::transport::Server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Config
    let config = Config::get()?;
    #[cfg(debug_assertions)]
    println!("{:#?}", config);

    // Logger
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator)
        .use_original_order()
        .build();
    let drain = LevelFilter::new(drain, config.log.level);
    let root = Logger::root(Mutex::new(drain).fuse(), o!());
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
