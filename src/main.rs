use anyhow::Result;
use desk::{config::Config, controllers, desk::Desk, logging, utils::Position};
use slog::{o, Drain, LevelFilter, Logger};
use std::sync::Mutex;

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
    controller.move_to(Position::from_cm(68.0)?).await?;
    loop {
        controller.update().await?;
    }
}
