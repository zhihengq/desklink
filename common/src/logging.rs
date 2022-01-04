use once_cell::sync::OnceCell;

use slog::Logger;

static LOG: OnceCell<Logger> = OnceCell::new();

pub fn set(logger: Logger) {
    LOG.set(logger).expect("logger is already initialized");
}

pub fn get() -> &'static Logger {
    LOG.get().expect("logger is not initialized")
}

macro_rules! create_macro {
    ($dollar:tt $logf:tt) => {
        #[macro_export]
        #[allow(unused)]
        macro_rules! $logf {
            ($dollar format:literal $dollar($dollar additional:tt)*) => {
                slog::$logf!(desk_common::logging::get(), concat!("[{}:{}] ", $dollar format), file!(), line!()
                    $dollar($dollar additional)*)
            };
        }
    };
}

pub(crate) use create_macro;
