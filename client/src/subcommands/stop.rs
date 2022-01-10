use crate::Client;
use desklink_common::rpc::{StopRequest, StopResponse};
use tonic::Status;

pub(crate) async fn run(mut client: Client) -> Result<(), Status> {
    let StopResponse {} = client.stop(StopRequest {}).await?.into_inner();
    Ok(())
}
