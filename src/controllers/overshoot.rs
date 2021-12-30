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

    async fn move_up_to(&mut self, position: Position) -> Result<(), ControllerError> {
        let mut interval = time::interval(Duration::from_millis(500));
        while self.desk.position < position {
            select! {
                _ = interval.tick() => self.desk.move_up().await?,
                result = self.desk.update() => {
                    result?;
                    if i16::from(&self.desk.velocity) == 0 {
                        return Err(ControllerError::Aborted);
                    }
                }
            }
        }
        self.desk.stop().await?;
        Ok(())
    }

    async fn move_down_to(&mut self, position: Position) -> Result<(), ControllerError> {
        let mut interval = time::interval(Duration::from_millis(500));
        while self.desk.position > position {
            select! {
                _ = interval.tick() => self.desk.move_down().await?,
                result = self.desk.update() => {
                    result?;
                    if i16::from(&self.desk.velocity) == 0 {
                        return Err(ControllerError::Aborted);
                    }
                }
            }
        }
        self.desk.stop().await?;
        Ok(())
    }
}
