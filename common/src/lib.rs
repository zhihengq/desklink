pub const PROJECT_NAME: &str = "desklink";

pub mod rpc {
    tonic::include_proto!("desk_service");
}

pub mod logging;
logging::create_macro!($ critical);
logging::create_macro!($ error);
logging::create_macro!($ warn);
logging::create_macro!($ info);
logging::create_macro!($ debug);
logging::create_macro!($ trace);
