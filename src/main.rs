use anyhow::{anyhow, Result};
use btleplug::api::BDAddr;
use desk::{
    desk::Desk,
    logging,
    utils::{Position, Velocity, UUID_STATE},
};
use slog::{o, Drain, Level, LevelFilter, Logger};
use std::{str::FromStr, sync::Mutex};
use structopt::StructOpt;

const DESK_ADDR: &str = "D6:A7:B1:F8:0F:79";

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short = "v", long = "log-level", default_value = "Info", parse(try_from_str = parse_log_level))]
    log_level: Level,
}

fn parse_log_level(name: &str) -> Result<Level> {
    Level::from_str(name).map_err(|()| anyhow!(format!("Invalid log level: {}", name)))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // command line options
    let opt = Opt::from_args();

    // create logger
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build();
    let drain = LevelFilter::new(drain, opt.log_level);
    let root = Logger::root(Mutex::new(drain).fuse(), o!());
    logging::set(root);

    let mut desk = Desk::find(BDAddr::from_str_delim(DESK_ADDR)?).await?;
    loop {
        desk.update().await?;
    }
}
