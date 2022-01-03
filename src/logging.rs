use once_cell::sync::OnceCell;

use slog::Logger;

static LOG: OnceCell<Logger> = OnceCell::new();

pub fn set(logger: Logger) {
    LOG.set(logger).expect("logger is already initialized");
}

pub(crate) fn get() -> &'static Logger {
    LOG.get().expect("logger is not initialized")
}

macro_rules! create_macro {
    ($dollar:tt $logf:tt) => {
        #[allow(unused)]
        macro_rules! $logf {
            ($dollar format:literal $dollar($dollar additional:tt)*) => {
                slog::$logf!(logging::get(), concat!("[{}:{}] ", $dollar format), file!(), line!()
                    $dollar($dollar additional)*)
            };
        }
        #[allow(unused)]
        pub(crate) use $logf;
    };
}

create_macro!($ critical);
create_macro!($ error);
create_macro!($ warning);
create_macro!($ info);
create_macro!($ debug);
create_macro!($ trace);
