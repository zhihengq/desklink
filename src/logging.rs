use once_cell::sync::OnceCell;

use slog::Logger;

static LOG: OnceCell<Logger> = OnceCell::new();

pub fn set(logger: Logger) {
    LOG.set(logger).expect("logger is already initialized");
}

pub fn get() -> &'static Logger {
    LOG.get().expect("logger is not initialized")
}
