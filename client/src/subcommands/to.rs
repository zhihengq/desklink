use crate::{Client, Position, Velocity};
use desklink_common::rpc::{StartMoveRequest, StartMoveResponse, SubscribeStateRequest};
use tonic::Status;
use tracing::info;

pub(crate) async fn run(mut client: Client, target: f32, wait: bool) -> Result<(), Status> {
    let states = if wait {
        Some(
            client
                .subscribe_state(SubscribeStateRequest {})
                .await?
                .into_inner(),
        )
    } else {
        None
    };

    let StartMoveResponse {} = client
        .start_move(StartMoveRequest { target })
        .await?
        .into_inner();

    if let Some(mut states) = states {
        while let Some(state) = states.message().await? {
            info!(
                position = state.position.cm(),
                velocity = state.velocity.cm_per_s(),
                "Update",
            );
            if f32::abs(state.position - target) < 0.1 {
                break;
            }
        }
    }

    Ok(())
}
