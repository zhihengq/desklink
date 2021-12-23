use anyhow::{anyhow, Result};
use desk::{logging, scan};
use slog::{o, Drain, Level, LevelFilter, Logger};
use std::{str::FromStr, sync::Mutex};
use structopt::StructOpt;

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

    scan().await?;
    Ok(())
}
