use lazy_static::lazy_static;

use slog::{o, Drain, Logger};
use std::sync::Mutex;

fn create_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = Mutex::new(slog_term::FullFormat::new(decorator).build()).fuse();
    Logger::root(drain, o!())
}

lazy_static! {
    pub static ref LOG: Logger = create_logger();
}
