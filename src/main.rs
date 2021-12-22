use anyhow::Result;
use desk::scan;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    scan().await?;
    Ok(())
}
