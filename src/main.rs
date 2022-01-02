use anyhow::Result;
use desk::{
    config::Config,
    controllers::{self, Command, Message},
    desk::Desk,
    logging,
    utils::Position,
};
use slog::{o, Drain, LevelFilter, Logger};
use std::sync::Mutex;
use tokio::sync::watch;

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
    let (inputs, rx) = watch::channel(Mutex::new(None));
    let j = tokio::spawn(async move {
        controller
            .drive(rx)
            .await
            .unwrap_or_else(|e| panic!("{}", e))
    });

    use tokio::io::AsyncBufReadExt;
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    loop {
        let user_input = lines.next_line().await?;
        if let Some(user_input) = user_input {
            let command = match user_input.trim().parse::<f32>() {
                Ok(cm) => Command::MoveTo {
                    target: Position::from_cm(cm)?,
                },
                Err(_) => Command::Stop,
            };
            let (m, complete) = Message::new(command);
            inputs
                .send(Mutex::new(Some(m)))
                .expect("Complete channel closed by sender");
            complete.await??;
        } else {
            break;
        }
    }

    drop(inputs);
    j.await?;

    Ok(())
}
