use anyhow::{anyhow, Result};
use btleplug::api::BDAddr;
use desk::{controllers, desk::Desk, logging, utils::Position};
use slog::{o, Drain, Level, LevelFilter, Logger};
use std::{str::FromStr, sync::Mutex};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(short = "v", long, default_value = "Info", parse(try_from_str = parse_log_level))]
    log_level: Level,

    #[structopt(short, long)]
    desk: BDAddr,
}

fn parse_log_level(name: &str) -> Result<Level> {
    Level::from_str(name).map_err(|()| anyhow!(format!("Invalid log level: '{}'", name)))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // command line options
    let args = Args::from_args();

    // create logger
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build();
    let drain = LevelFilter::new(drain, args.log_level);
    let root = Logger::root(Mutex::new(drain).fuse(), o!());
    logging::set(root);

    let desk = Desk::find(args.desk).await?;
    let mut controller = controllers::create_controller(desk);
    controller.move_to(Position::from_cm(68.0)?).await?;
    loop {
        controller.update().await?;
    }
}
