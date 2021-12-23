pub mod logging;

use anyhow::{anyhow, Result};
use btleplug::{
    api::{Central, Manager as _},
    platform::Manager,
};
use std::time::Duration;
use tokio::time;

pub async fn scan() -> Result<()> {
    let manager = Manager::new().await?;
    let central = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or(anyhow!("no adaptors"))?;
    central.start_scan(Default::default()).await?;
    time::sleep(Duration::from_secs(5)).await;
    central.stop_scan().await?;
    let devices = central.peripherals().await?;
    slog::info!(logging::get(), "Scanned devices"; "devices" => #?devices);
    Ok(())
}
