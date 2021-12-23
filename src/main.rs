use anyhow::Result;
use btleplug::api::BDAddr;
use desk::{
    desk::Desk,
    utils::{Position, Velocity, UUID_STATE},
};

const DESK_ADDR: &str = "D6:A7:B1:F8:0F:79";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let mut desk = Desk::find(BDAddr::from_str_delim(DESK_ADDR)?).await?;
    loop {
        desk.update().await?;
    }
}
