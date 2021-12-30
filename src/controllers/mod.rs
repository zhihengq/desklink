mod overshoot;

use crate::{
    desk::{Desk, DeskError},
    logging,
    utils::Position,
};
use async_trait::async_trait;
use slog::debug;
use std::cmp::Ordering;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error(transparent)]
    DeskError(#[from] DeskError),
    #[error("Aborted by user")]
    Aborted,
}

#[async_trait]
pub trait Controller: Send {
    fn desk(&mut self) -> &mut Desk;
    async fn move_up_to(&mut self, position: Position) -> Result<(), ControllerError>;
    async fn move_down_to(&mut self, position: Position) -> Result<(), ControllerError>;

    async fn move_to(&mut self, position: Position) -> Result<(), ControllerError> {
        debug!(logging::get(), "moving to {}", position);
        let current_position = self.desk().position;
        match Ord::cmp(&position, &current_position) {
            Ordering::Equal => Ok(()),
            Ordering::Less => self.move_down_to(position).await,
            Ordering::Greater => self.move_up_to(position).await,
        }
    }

    async fn update(&mut self) -> Result<(), ControllerError> {
        Ok(self.desk().update().await?)
    }
}

pub fn create_controller(desk: Desk) -> Box<dyn Controller + Send> {
    Box::new(overshoot::OvershootController::new(desk))
}
