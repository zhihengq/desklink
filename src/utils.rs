use std::fmt::{Display, Error, Formatter};

pub const UUID_STATE: &str = "99fa0021-338a-1024-8a49-009c0215f78a";
pub const UUID_COMMAND: &str = "99fa0002-338a-1024-8a49-009c0215f78a";
pub const UUID_REFERENCE_INPUT: &str = "99fa0031-338a-1024-8a49-009c0215f78a";

pub const COMMAND_DOWN: [u8; 2] = [0x46, 0x00];
pub const COMMAND_UP: [u8; 2] = [0x47, 0x00];
pub const COMMAND_STOP: [u8; 2] = [0xff, 0x00];

pub const COMMAND_REFERENCE_INPUT_DOWN: [u8; 2] = [0xff, 0x7f];
pub const COMMAND_REFERENCE_INPUT_UP: [u8; 2] = [0x00, 0x80];
pub const COMMAND_REFERENCE_INPUT_STOP: [u8; 2] = [0x01, 0x80];

pub struct Position(u16);

impl From<[u8; 2]> for Position {
    fn from(raw: [u8; 2]) -> Self {
        Position(u16::from_le_bytes(raw))
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
        write!(f, "{:.2} cm", (height as f64) / 100.0)
    }
}

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
        write!(f, "{:>6.3} cm/s", (velocity as f64) / 1000.0)
    }
}
