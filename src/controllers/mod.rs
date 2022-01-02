mod overshoot;

use crate::{
    desk::{Desk, DeskError},
    logging,
    utils::Position,
};
use async_trait::async_trait;
use slog::{error, info};
use std::{cmp::Ordering, future::Future, pin::Pin, ptr::NonNull, sync::Mutex};
use thiserror::Error;
use tokio::{
    select,
    sync::{oneshot, watch},
};

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error(transparent)]
    DeskError(#[from] DeskError),

    #[error("Aborted by user")]
    Aborted,
}

#[derive(Debug)]
pub enum Command {
    Stop,
    MoveTo { target: Position },
}

#[derive(Debug)]
pub struct Message {
    command: Command,
    complete: oneshot::Sender<Result<(), ControllerError>>,
}

impl Message {
    pub fn new(command: Command) -> (Message, oneshot::Receiver<Result<(), ControllerError>>) {
        let (tx, rx) = oneshot::channel();
        let message = Message {
            command,
            complete: tx,
        };
        (message, rx)
    }
}

#[async_trait]
pub trait Controller: Send {
    fn desk(&mut self) -> &mut Desk;
    async fn move_up_to(&mut self, position: Position) -> Result<(), ControllerError>;
    async fn move_down_to(&mut self, position: Position) -> Result<(), ControllerError>;

    async fn move_to(&mut self, position: Position) -> Result<(), ControllerError> {
        info!(logging::get(), "Moving to {}", position);
        let current_position = self.desk().position;
        let result = match Ord::cmp(&position, &current_position) {
            Ordering::Equal => Ok(()),
            Ordering::Less => self.move_down_to(position).await,
            Ordering::Greater => self.move_up_to(position).await,
        };
        result.map_err(|e| {
            error!(
                logging::get(),
                "Error during Controller::move_to({}): {}", position, e
            );
            e
        })
    }

    async fn stop(&mut self) -> Result<(), ControllerError> {
        info!(logging::get(), "Stopping");
        self.desk().stop().await.map_err(|e| {
            let e = e.into();
            error!(logging::get(), "Error during Controller::stop(): {}", e);
            e
        })
    }

    async fn update(&mut self) -> Result<(), ControllerError> {
        self.desk().update().await.map_err(|e| {
            let e = e.into();
            error!(logging::get(), "Error during Controller::update(): {}", e);
            e
        })
    }

    async fn drive(
        &mut self,
        mut inputs: watch::Receiver<Mutex<Option<Message>>>,
    ) -> Result<(), ControllerError> {
        let mut in_progress: Option<
            Pin<Box<dyn Future<Output = Result<(), ControllerError>> + Send>>,
        > = None;
        let mut self_ptr = SendPtr::new(self);

        loop {
            match in_progress.take() {
                Some(mut task) => {
                    select! {
                        result = &mut task => result?,
                        result = inputs.changed() => {
                            if result.is_err() {
                                return task.await;
                            } else {
                                drop(task); // must drop future before borrowing self
                                let Message { command, complete } = inputs
                                    .borrow_and_update()
                                    .lock()
                                    .expect("Poisoned mutex")
                                    .take()
                                    .expect("No message");
                                match command {
                                    Command::Stop => {
                                        let result = self.stop().await;
                                        complete.send(result)
                                            .expect("Complete channel closed by receiver");
                                    }
                                    Command::MoveTo { target } => {
                                        in_progress = Some(
                                            // future must be dropped before borrowing self
                                            unsafe { self_ptr.as_mut() }.move_to(target)
                                        );
                                        complete.send(Ok(()))
                                            .expect("Complete channel closed by receiver");
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    select! {
                        result = self.update() => result?,
                        result = inputs.changed() => {
                            if result.is_err() {
                                // not more inputs
                                return Ok(());
                            } else {
                                let Message {command, complete} = inputs
                                    .borrow_and_update()
                                    .lock()
                                    .expect("Poisoned mutex")
                                    .take()
                                    .expect("No message");
                                match command {
                                    Command::Stop => {
                                        let result = self.stop().await;
                                        complete.send(result)
                                            .expect("Complete channel closed by receiver");
                                    }
                                    Command::MoveTo {target} => {
                                        in_progress = Some(
                                            // future must be dropped before borrowing self
                                            unsafe { self_ptr.as_mut() }.move_to(target)
                                        );
                                        complete.send(Ok(()))
                                            .expect("Complete channel closed by receiver");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
struct SendPtr<T: ?Sized>(NonNull<T>);
unsafe impl<T: ?Sized> Send for SendPtr<T> {}
impl<T: ?Sized> SendPtr<T> {
    fn new(r: &mut T) -> Self {
        SendPtr(unsafe { NonNull::new_unchecked(r) })
    }
    unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        self.0.as_mut()
    }
}

pub fn create_controller(desk: Desk) -> Box<dyn Controller + Send> {
    Box::new(overshoot::OvershootController::new(desk))
}
