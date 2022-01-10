use std::fmt::{Display, Error, Formatter};
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum PositionError {
    #[error("Position out of bound: {0:.2} cm")]
    OutOfBound(f32),
    #[error("Invalid position: {0}")]
    InvalidPosition(String),
}

/**
 * Position is represented by an u16 of position ticks.
 * Each position tick is 1/10 millimeters.
 */
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

    pub fn to_cm(self) -> f32 {
        (self.0 as u16 + 6200) as f32 / 100.0
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

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{:>6.2} cm", self.to_cm())
    }
}

/**
 * Velocity is represented by an i16 of velocity ticks.
 * Each velocity tick is 1/100 millimeters.
 */
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct Velocity(i16);

impl Velocity {
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn to_cm_per_s(&self) -> f32 {
        self.0 as f32 / 1000.0
    }
}

impl From<[u8; 2]> for Velocity {
    fn from(raw: [u8; 2]) -> Self {
        Velocity(i16::from_le_bytes(raw))
    }
}

impl Display for Velocity {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{:>6.3} cm/s", self.to_cm_per_s())
    }
}
