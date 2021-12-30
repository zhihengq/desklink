use std::fmt::{Display, Error, Formatter};

pub const UUID_STATE: &str = "99fa0021-338a-1024-8a49-009c0215f78a";
pub const UUID_COMMAND: &str = "99fa0002-338a-1024-8a49-009c0215f78a";
// Update this characteristic to match app behavior
//pub const UUID_REFERENCE_INPUT: &str = "99fa0031-338a-1024-8a49-009c0215f78a";

pub const COMMAND_DOWN: [u8; 2] = [0x46, 0x00];
pub const COMMAND_UP: [u8; 2] = [0x47, 0x00];
pub const COMMAND_STOP: [u8; 2] = [0xff, 0x00];

//pub const COMMAND_REFERENCE_INPUT_DOWN: [u8; 2] = [0xff, 0x7f];
//pub const COMMAND_REFERENCE_INPUT_UP: [u8; 2] = [0x00, 0x80];
//pub const COMMAND_REFERENCE_INPUT_STOP: [u8; 2] = [0x01, 0x80];

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PositionError {
    #[error("Position out of bound: {0:.2} cm")]
    OutOfBound(f32),
    #[error("Invalid position: {0}")]
    InvalidPosition(String),
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Position(u16);

impl Position {
    pub fn from_cm(cm: f32) -> Result<Self, PositionError> {
        if !(62.0..=127.0).contains(&cm) {
            return Err(PositionError::OutOfBound(cm));
        }
        let pos = Position((cm * 100.0) as u16 - 6200);
        pos.check()?;
        Ok(pos)
    }

    fn check(&self) -> Result<(), PositionError> {
        if self.0 > 6500 {
            Err(PositionError::InvalidPosition(format!("{}", self)))
        } else {
            Ok(())
        }
    }
}

impl TryFrom<[u8; 2]> for Position {
    type Error = PositionError;
    fn try_from(raw: [u8; 2]) -> Result<Self, Self::Error> {
        let pos = Position(u16::from_le_bytes(raw));
        pos.check()?;
        Ok(pos)
    }
}

impl From<&Position> for u16 {
    fn from(pos: &Position) -> Self {
        pos.0 as u16 + 6200
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let height: u16 = self.into();
        write!(f, "{:.2} cm", (height as f32) / 100.0)
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Velocity(i16);

impl From<[u8; 2]> for Velocity {
    fn from(raw: [u8; 2]) -> Self {
        Velocity(i16::from_le_bytes(raw))
    }
}

impl From<&Velocity> for i16 {
    fn from(vel: &Velocity) -> Self {
        vel.0
    }
}

impl Display for Velocity {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let velocity: i16 = self.into();
        write!(f, "{:>6.3} cm/s", (velocity as f32) / 1000.0)
    }
}
