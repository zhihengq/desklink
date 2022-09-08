use crate::utils::{
    Position, PositionError, Velocity, COMMAND_DOWN, COMMAND_STOP, COMMAND_UP, UUID_COMMAND,
    UUID_STATE,
};
use btleplug::{
    api::{
        BDAddr, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _,
        ValueNotification, WriteType,
    },
    platform::{Manager, Peripheral},
};
use desklink_common::{debug, error, trace};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use thiserror::Error;
use tokio::sync::watch;

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
    // bluetooth
    device: Peripheral,
    events: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    command_characteristic: Characteristic,
    // desk state
    pub state: watch::Receiver<(Position, Velocity)>,
    state_publisher: watch::Sender<(Position, Velocity)>,
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
                trace!("Discovered device: {:?}", id);
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
            .find(|c| c.uuid.hyphenated().to_string() == UUID_STATE)
            .ok_or(DeskError::CharacteristicNotFound {
                purpose: "state",
                uuid: UUID_STATE,
            })?;
        let char_command = characteristics
            .iter()
            .find(|c| c.uuid.hyphenated().to_string() == UUID_COMMAND)
            .ok_or(DeskError::CharacteristicNotFound {
                purpose: "command",
                uuid: UUID_COMMAND,
            })?;

        // event subscription
        device.subscribe(char_state).await?;
        let events = device.notifications().await?;

        // state notification
        let raw_state = device.read(char_state).await?;
        let (position, velocity) = Self::parse_state(raw_state)?;
        debug!("Initial state"; "position" => %position, "velocity" => %velocity);
        let (tx, rx) = watch::channel((position, velocity));

        Ok(Desk {
            device,
            events,
            command_characteristic: char_command.clone(),
            state: rx,
            state_publisher: tx,
        })
    }

    pub async fn move_up(&mut self) -> Result<(), DeskError> {
        trace!("Sending bluetooth command: up");
        self.device
            .write(
                &self.command_characteristic,
                &COMMAND_UP,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn move_down(&mut self) -> Result<(), DeskError> {
        trace!("Sending bluetooth command: down");
        self.device
            .write(
                &self.command_characteristic,
                &COMMAND_DOWN,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), DeskError> {
        trace!("Sending bluetooth command: stop");
        self.device
            .write(
                &self.command_characteristic,
                &COMMAND_STOP,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(Position, Velocity), DeskError> {
        let event = self.events.next().await.expect("No more events");
        assert!(event.uuid.hyphenated().to_string() == UUID_STATE);
        let raw_state = event.value;
        let (position, velocity) = Self::parse_state(raw_state)?;
        debug!("Updated state"; "position" => %position, "velocity" => %velocity);
        self.state_publisher.send_replace((position, velocity));
        Ok((position, velocity))
    }

    pub fn state(&self) -> (Position, Velocity) {
        *self.state.borrow()
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
