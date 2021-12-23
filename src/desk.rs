use crate::{
    logging,
    utils::{
        Position, Velocity, COMMAND_DOWN, COMMAND_STOP, COMMAND_UP, UUID_COMMAND,
        UUID_REFERENCE_INPUT, UUID_STATE,
    },
};
use anyhow::{anyhow, Result};
use btleplug::{
    api::{
        BDAddr, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _,
        ValueNotification, WriteType,
    },
    platform::{Manager, Peripheral},
};
use futures::stream::{Stream, StreamExt};
use slog::{debug, info};
use std::pin::Pin;

pub struct Desk {
    device: Peripheral,
    position: Position,
    velocity: Velocity,
    events: Pin<Box<dyn Stream<Item = ValueNotification>>>,
    char_command: Characteristic,
    // char_reference_input: Characteristic,
}

impl Desk {
    pub async fn find(address: BDAddr) -> Result<Desk> {
        // setup local central
        let manager = Manager::new().await?;
        let central = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("no adaptors"))?;
        central.start_scan(Default::default()).await?;

        // find target peripheral
        let mut events = central.events().await?;
        let id = loop {
            let event = events.next().await.expect("No more events");
            if let CentralEvent::DeviceDiscovered(id) = event {
                debug!(logging::get(), "discovered device: {:?}", id);
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
            .ok_or_else(|| anyhow!("Cannot find characteristic for state: {}", UUID_STATE))?;
        let char_command = characteristics
            .iter()
            .find(|c| c.uuid.to_hyphenated().to_string() == UUID_COMMAND)
            .ok_or_else(|| anyhow!("Cannot find characteristic for command: {}", UUID_COMMAND))?;

        // initial state
        let raw_state = device.read(char_state).await?;
        let (position, velocity) = Self::parse_state(raw_state);
        info!(logging::get(), "initial state"; "position" => %position, "velocity" => %velocity);

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

    pub async fn move_up(&self) -> Result<()> {
        self.device
            .write(&self.char_command, &COMMAND_UP, WriteType::WithoutResponse)
            .await?;
        Ok(())
    }

    pub async fn move_down(&self) -> Result<()> {
        self.device
            .write(
                &self.char_command,
                &COMMAND_DOWN,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.device
            .write(
                &self.char_command,
                &COMMAND_STOP,
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn update(&mut self) -> Result<()> {
        let event = self.events.next().await.ok_or(anyhow!("No more events"))?;
        assert!(event.uuid.to_hyphenated().to_string() == UUID_STATE);
        let raw_state = event.value;
        let (position, velocity) = Self::parse_state(raw_state);
        info!(logging::get(), "updated state"; "position" => %position, "velocity" => %velocity);
        self.position = position;
        self.velocity = velocity;
        Ok(())
    }

    fn parse_state(raw_state: Vec<u8>) -> (Position, Velocity) {
        assert!(raw_state.len() == 4);
        let raw_position: [u8; 2] = raw_state[0..2].try_into().unwrap();
        let raw_velocity: [u8; 2] = raw_state[2..4].try_into().unwrap();
        let position = Position::from(raw_position);
        let velocity = Velocity::from(raw_velocity);
        (position, velocity)
    }
}
