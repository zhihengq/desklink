use crate::{
    logging,
    utils::{
        Position, PositionError, Velocity, COMMAND_DOWN, COMMAND_STOP, COMMAND_UP, UUID_COMMAND,
        UUID_STATE,
    },
};
use btleplug::{
    api::{
        BDAddr, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _,
        ValueNotification, WriteType,
    },
    platform::{Manager, Peripheral},
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeskError {
    #[error("Bluetooth error: {0}")]
    BluetoothError(#[from] btleplug::Error),

    #[error("No bluetooth adaptor")]
    NoBluetoothAdaptor,

    #[error("Cannot find bluetooth characteristic for {purpose}: {uuid}")]
    CharacteristicNotFound {
        purpose: &'static str,
        uuid: &'static str,
    },

    #[error(transparent)]
    InvalidPosition(#[from] PositionError),
}

pub struct Desk {
    device: Peripheral,
    pub position: Position,
    pub velocity: Velocity,
    events: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    char_command: Characteristic,
}

impl Desk {
    pub async fn find(address: BDAddr) -> Result<Desk, DeskError> {
        // setup local central
        let manager = Manager::new().await?;
        let central = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or(DeskError::NoBluetoothAdaptor)?;
        central.start_scan(Default::default()).await?;

        // find target peripheral
        let mut events = central.events().await?;
        let id = loop {
            let event = events.next().await.expect("No more events");
            if let CentralEvent::DeviceDiscovered(id) = event {
                logging::trace!("Discovered device: {:?}", id);
                if central.peripheral(&id).await?.address() == address {
                    break id;
                }
            }
        };

        // setup target connection
        let device = central.peripheral(&id).await?;
        device.connect().await?;
        device.discover_services().await?;

        // characteristics
        let characteristics = device.characteristics();
        let char_state = characteristics
            .iter()
            .find(|c| c.uuid.to_hyphenated().to_string() == UUID_STATE)
            .ok_or(DeskError::CharacteristicNotFound {
                purpose: "state",
                uuid: UUID_STATE,
            })?;
        let char_command = characteristics
            .iter()
            .find(|c| c.uuid.to_hyphenated().to_string() == UUID_COMMAND)
            .ok_or(DeskError::CharacteristicNotFound {
                purpose: "command",
                uuid: UUID_COMMAND,
            })?;

        // initial state
        let raw_state = device.read(char_state).await?;
        let (position, velocity) = Self::parse_state(raw_state)?;
        logging::debug!("Initial state"; "position" => %position, "velocity" => %velocity);

        // event subscription
        device.subscribe(char_state).await?;
        let events = device.notifications().await?;

        Ok(Desk {
            device,
            position,
            velocity,
            events,
            char_command: char_command.clone(),
        })
    }

    pub async fn move_up(&mut self) -> Result<(), DeskError> {
        logging::trace!("Sending bluetooth command: up");
        self.device
            .write(&self.char_command, &COMMAND_UP, WriteType::WithoutResponse)
            .await?;
        Ok(())
    }

    pub async fn move_down(&mut self) -> Result<(), DeskError> {
        logging::trace!("Sending bluetooth command: down");
        self.device
            .write(
                &self.char_command,
                &COMMAND_DOWN,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), DeskError> {
        logging::trace!("Sending bluetooth command: stop");
        self.device
            .write(
                &self.char_command,
                &COMMAND_STOP,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), DeskError> {
        let event = self.events.next().await.expect("No more events");
        assert!(event.uuid.to_hyphenated().to_string() == UUID_STATE);
        let raw_state = event.value;
        let (position, velocity) = Self::parse_state(raw_state)?;
        logging::debug!("Updated state"; "position" => %position, "velocity" => %velocity);
        self.position = position;
        self.velocity = velocity;
        Ok(())
    }

    fn parse_state(raw_state: Vec<u8>) -> Result<(Position, Velocity), DeskError> {
        assert!(raw_state.len() == 4);
        let raw_position: [u8; 2] = raw_state[0..2].try_into().unwrap();
        let raw_velocity: [u8; 2] = raw_state[2..4].try_into().unwrap();
        let position = Position::try_from(raw_position)?;
        let velocity = Velocity::from(raw_velocity);
        Ok((position, velocity))
    }
}
