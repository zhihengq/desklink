use anyhow::Result;
use desk_common::logging;
use desk_server::{
    config::Config,
    controllers,
    desk::Desk,
    service::{DeskService, DeskServiceServer},
};
use slog::{o, Drain, LevelFilter, Logger};
use std::sync::Mutex;
use tokio::sync::watch;
use tonic::transport::Server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Config
    let config = Config::get()?;
    println!("{:#?}", config);

    // Logger
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build();
    let drain = LevelFilter::new(drain, config.log.level);
    let root = Logger::root(Mutex::new(drain).fuse(), o!());
    logging::set(root);

    // Desk control
    let desk = Desk::find(config.desk.address).await?;
    let mut controller = controllers::create_controller(desk);
    let (tx, rx) = watch::channel(Default::default());
    let join_controller = tokio::spawn(async move {
        controller
            .drive(rx)
            .await
            .unwrap_or_else(|e| panic!("{}", e))
    });

    let svc = DeskServiceServer::new(DeskService::new(tx));
    Server::builder()
        .add_service(svc)
        .serve(config.server.address)
        .await?;

    join_controller.await?;
    Ok(())
}
