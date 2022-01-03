mod overshoot;

use crate::{
    desk::{Desk, DeskError},
    logging,
    utils::Position,
};
use async_trait::async_trait;
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

pub fn create_controller(desk: Desk) -> Box<dyn Controller> {
    Box::new(overshoot::OvershootController::new(desk))
}

#[async_trait]
pub trait Controller: Send {
    fn desk(&mut self) -> &mut Desk;
    async fn move_up_to(&mut self, position: Position) -> Result<(), ControllerError>;
    async fn move_down_to(&mut self, position: Position) -> Result<(), ControllerError>;

    async fn move_to(&mut self, position: Position) -> Result<(), ControllerError> {
        logging::trace!("Start moving to {}", position);
        let current_position = self.desk().position;
        let result = match Ord::cmp(&position, &current_position) {
            Ordering::Equal => Ok(()),
            Ordering::Less => self.move_down_to(position).await,
            Ordering::Greater => self.move_up_to(position).await,
        };
        match &result {
            Ok(()) => logging::trace!("Finish moving to {}", position),
            Err(e) => logging::error!("Error moving to {}: {}", position, e),
        }
        result
    }

    async fn stop(&mut self) -> Result<(), ControllerError> {
        logging::trace!("Start stopping");
        let result = self.desk().stop().await.map_err(Into::into);
        match &result {
            Ok(()) => logging::trace!("Finish stopping"),
            Err(e) => logging::error!("Error stopping: {}", e),
        }
        result
    }

    async fn update(&mut self) -> Result<(), ControllerError> {
        self.desk().update().await.map_err(|e| {
            let e = e.into();
            logging::error!("Error updateing: {}", e);
            e
        })
    }

    async fn drive(
        &mut self,
        mut inputs: watch::Receiver<Mutex<Option<Message>>>,
    ) -> Result<(), ControllerError> {
        let mut in_progress: InProgress<'_> = None;

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
                                unsafe {
                                    process_command(self, &mut inputs, &mut in_progress)
                                }.await;
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
                                unsafe {
                                    process_command(self, &mut inputs, &mut in_progress)
                                }.await;
                            }
                        }
                    }
                }
            }
        }
    }
}

type InProgress<'a> =
    Option<Pin<Box<dyn Future<Output = Result<(), ControllerError>> + Send + 'a>>>;

// The future stored in `in_progress` must be dropped before self is borrowed again
async unsafe fn process_command<'a, C: Controller + ?Sized + 'a>(
    controller: &mut C,
    inputs: &mut watch::Receiver<Mutex<Option<Message>>>,
    in_progress: &mut InProgress<'a>,
) {
    let mut self_ptr = SendPtr::new(controller);
    let Message { command, complete } = inputs
        .borrow_and_update()
        .lock()
        .expect("Poisoned mutex")
        .take()
        .expect("No message");
    match command {
        Command::Stop => {
            let result = controller.stop().await;
            complete
                .send(result)
                .expect("Complete channel closed by receiver");
        }
        Command::MoveTo { target } => {
            *in_progress = Some(
                // future must be dropped before borrowing self
                unsafe { self_ptr.as_mut() }.move_to(target),
            );
            complete
                .send(Ok(()))
                .expect("Complete channel closed by receiver");
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
