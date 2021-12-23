use crate::{
    logging,
    utils::{Position, Velocity, UUID_COMMAND, UUID_REFERENCE_INPUT, UUID_STATE},
};
use anyhow::{anyhow, Result};
use btleplug::{
    api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral as _, ValueNotification},
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
}

impl Desk {
    pub async fn find(address: BDAddr) -> Result<Desk> {
        let manager = Manager::new().await?;
        let central = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("no adaptors"))?;
        central.start_scan(Default::default()).await?;

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

        let device = central.peripheral(&id).await?;
        device.connect().await?;
        device.discover_services().await?;

        let characteristics = device.characteristics();

        let char_state = characteristics
            .iter()
            .find(|c| c.uuid.to_hyphenated().to_string() == UUID_STATE)
            .unwrap();
        let raw_state = device.read(char_state).await?;
        let (position, velocity) = Self::parse_state(raw_state);
        info!(logging::get(), "initial state"; "position" => %position, "velocity" => %velocity);

        device.subscribe(char_state).await?;
        let events = device.notifications().await?;

        Ok(Desk {
            device,
            position,
            velocity,
            events,
        })
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
