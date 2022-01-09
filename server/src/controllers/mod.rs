mod overshoot;

use crate::{
    desk::{Desk, DeskError},
    utils::{Position, Velocity},
};
use async_trait::async_trait;
use desklink_common::{error, trace};
use futures::Stream;
use std::{cmp::Ordering, future::Future, pin::Pin, ptr::NonNull, sync::Mutex};
use thiserror::Error;
use tokio::{
    select,
    sync::{oneshot, watch},
};
use tokio_stream::wrappers::WatchStream;

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error(transparent)]
    DeskError(#[from] DeskError),

    #[error("Aborted by user")]
    Aborted,
}

type CompletePromise<T> = oneshot::Sender<Result<T, ControllerError>>;
pub type Complete<T> = oneshot::Receiver<Result<T, ControllerError>>;
pub type CommandReceiver = watch::Receiver<Mutex<Option<Command>>>;
pub type CommandSender = watch::Sender<Mutex<Option<Command>>>;
pub type StateStream = Pin<Box<dyn Stream<Item = (Position, Velocity)> + Send>>;

pub trait CommandSenderExt {
    fn send_command(&self, command: Command);
}

impl CommandSenderExt for CommandSender {
    fn send_command(&self, command: Command) {
        self.send_replace(Mutex::new(Some(command)));
    }
}

pub enum Command {
    GetState {
        result: CompletePromise<(Position, Velocity)>,
    },
    SubscribeState {
        result: CompletePromise<StateStream>,
    },
    Stop {
        complete: CompletePromise<()>,
    },
    MoveTo {
        target: Position,
        complete: CompletePromise<()>,
    },
}

impl Command {
    pub fn get_state() -> (Command, Complete<(Position, Velocity)>) {
        let (tx, rx) = oneshot::channel();
        (Command::GetState { result: tx }, rx)
    }

    pub fn subscribe_state() -> (Command, Complete<StateStream>) {
        let (tx, rx) = oneshot::channel();
        (Command::SubscribeState { result: tx }, rx)
    }

    pub fn stop() -> (Command, Complete<()>) {
        let (tx, rx) = oneshot::channel();
        (Command::Stop { complete: tx }, rx)
    }

    pub fn move_to(target: Position) -> (Command, Complete<()>) {
        let (tx, rx) = oneshot::channel();
        (
            Command::MoveTo {
                target,
                complete: tx,
            },
            rx,
        )
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
        trace!("Start moving to {}", position);
        let current_position = self.desk().state().0;
        let result = match Ord::cmp(&position, &current_position) {
            Ordering::Equal => Ok(()),
            Ordering::Less => self.move_down_to(position).await,
            Ordering::Greater => self.move_up_to(position).await,
        };
        match &result {
            Ok(()) => trace!("Finish moving to {}", position),
            Err(e) => error!("Error moving to {}: {}", position, e),
        }
        result
    }

    async fn stop(&mut self) -> Result<(), ControllerError> {
        trace!("Start stopping");
        let result = self.desk().stop().await.map_err(Into::into);
        match &result {
            Ok(()) => trace!("Finish stopping"),
            Err(e) => error!("Error stopping: {}", e),
        }
        result
    }

    async fn update(&mut self) -> Result<(Position, Velocity), ControllerError> {
        self.desk().update().await.map_err(|e| {
            let e = e.into();
            error!("Error updateing: {}", e);
            e
        })
    }

    async fn drive(&mut self, mut inputs: CommandReceiver) -> Result<(), ControllerError> {
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
                        result = self.update() => {
                            result?;
                        }
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
    inputs: &mut CommandReceiver,
    in_progress: &mut InProgress<'a>,
) {
    let mut self_ptr = SendPtr::new(controller);
    let command = inputs
        .borrow_and_update()
        .lock()
        .expect("Poisoned mutex")
        .take()
        .expect("No message");
    match command {
        Command::GetState { result } => {
            let state = controller.desk().state();
            result.send(Ok(state)).unwrap_or(());
        }
        Command::SubscribeState { result } => {
            let stream = Box::pin(WatchStream::new(controller.desk().state.clone()));
            result.send(Ok(stream)).unwrap_or(());
        }
        Command::Stop { complete } => {
            let result = controller.stop().await;
            complete.send(result).unwrap_or(());
        }
        Command::MoveTo { target, complete } => {
            *in_progress = Some(
                // future must be dropped before borrowing self
                unsafe { self_ptr.as_mut() }.move_to(target),
            );
            complete.send(Ok(())).unwrap_or(());
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
