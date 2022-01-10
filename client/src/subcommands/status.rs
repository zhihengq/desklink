use crate::{Client, Position, Velocity};
use desklink_common::rpc::{GetStateRequest, GetStateResponse};
use tonic::Status;

pub(crate) async fn run(mut client: Client) -> Result<(), Status> {
    let GetStateResponse { position, velocity } =
        client.get_state(GetStateRequest {}).await?.into_inner();
    println!(
        "Position: {}\nVelocity: {}",
        position.cm(),
        velocity.cm_per_s()
    );
    Ok(())
}
