use crate::{
    controllers::{Controller, ControllerError},
    desk::Desk,
    utils::Position,
};
use async_trait::async_trait;
use std::time::Duration;
use tokio::{select, time};

pub struct OvershootController {
    desk: Desk,
}

impl OvershootController {
    pub fn new(desk: Desk) -> Self {
        OvershootController { desk }
    }
}

#[async_trait]
impl Controller for OvershootController {
    fn desk(&mut self) -> &mut Desk {
        &mut self.desk
    }

    async fn move_up_to(&mut self, target: Position) -> Result<(), ControllerError> {
        let mut interval = time::interval(Duration::from_millis(500));
        let mut position = self.desk.state().0;
        while position < target {
            select! {
                _ = interval.tick() => self.desk.move_up().await?,
                result = self.desk.update() => {
                    let (_position, velocity) = result?;
                    position = _position;
                    if velocity.is_zero() {
                        return Err(ControllerError::Aborted);
                    }
                }
            }
        }
        self.desk.stop().await?;
        Ok(())
    }

    async fn move_down_to(&mut self, target: Position) -> Result<(), ControllerError> {
        let mut interval = time::interval(Duration::from_millis(500));
        let mut position = self.desk.state().0;
        while position > target {
            select! {
                _ = interval.tick() => self.desk.move_down().await?,
                result = self.desk.update() => {
                    let (_position, velocity) = result?;
                    position = _position;
                    if velocity.is_zero() {
                        return Err(ControllerError::Aborted);
                    }
                }
            }
        }
        self.desk.stop().await?;
        Ok(())
    }
}
