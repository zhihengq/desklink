mod overshoot;

use crate::{desk::Desk, logging, utils::Position};
use anyhow::Result;
use async_trait::async_trait;
use slog::debug;
use std::cmp::Ordering;

#[async_trait]
pub trait Controller: Send {
    fn desk(&mut self) -> &mut Desk;
    async fn move_up_to(&mut self, position: Position) -> Result<()>;
    async fn move_down_to(&mut self, position: Position) -> Result<()>;

    async fn move_to(&mut self, position: Position) -> Result<()> {
        debug!(logging::get(), "moving to {}", position);
        let current_position = self.desk().position;
        match Ord::cmp(&position, &current_position) {
            Ordering::Equal => Ok(()),
            Ordering::Less => self.move_down_to(position).await,
            Ordering::Greater => self.move_up_to(position).await,
        }
    }

    async fn update(&mut self) -> Result<()> {
        self.desk().update().await
    }
}

pub fn create_controller(desk: Desk) -> Box<dyn Controller + Send> {
    Box::new(overshoot::OvershootController::new(desk))
}
