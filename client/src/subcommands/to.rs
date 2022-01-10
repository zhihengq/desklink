use crate::Client;
use desklink_common::{
    info,
    rpc::{StartMoveRequest, StartMoveResponse, SubscribeStateRequest},
};
use tonic::Status;

pub(crate) async fn run(mut client: Client, target: f32, wait: bool) -> Result<(), Status> {
    let mut states = client
        .subscribe_state(SubscribeStateRequest {})
        .await?
        .into_inner();
    let StartMoveResponse {} = client
        .start_move(StartMoveRequest { target })
        .await?
        .into_inner();

    if wait {
        while let Some(state) = states.message().await? {
            info!(
                "update state";
                "position" => format!("{:.2} cm", state.position),
                "velocity" => format!("{:.3} cm/s", state.velocity),
            );
            if f32::abs(state.position - target) < 0.1 {
                break;
            }
        }
    }

    Ok(())
}