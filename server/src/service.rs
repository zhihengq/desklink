use crate::{
    controllers::{Command, CommandSender, CommandSenderExt, ControllerError},
    utils::{Position, PositionError},
};
use async_trait::async_trait;
pub use desklink_common::rpc::desk_service_server::DeskServiceServer;
use desklink_common::rpc::{
    desk_service_server::DeskService as DeskServiceTrait, GetStateRequest, GetStateResponse,
    StartMoveRequest, StartMoveResponse, StopRequest, StopResponse, SubscribeStateRequest,
    SubscribeStateResponse,
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tonic::{Request, Response, Status};
use tracing::info;

pub struct DeskService {
    controller: CommandSender,
}

impl DeskService {
    pub fn new(controller: CommandSender) -> Self {
        DeskService { controller }
    }
}

impl From<ControllerError> for Status {
    fn from(e: ControllerError) -> Status {
        match &e {
            ControllerError::DeskError(_) => Status::internal(format!("{}", e)),
            ControllerError::Aborted => Status::cancelled(format!("{}", e)),
        }
    }
}

#[async_trait]
impl DeskServiceTrait for DeskService {
    type SubscribeStateStream =
        Pin<Box<dyn Stream<Item = Result<SubscribeStateResponse, Status>> + Send>>;

    async fn get_state(
        &self,
        request: Request<GetStateRequest>,
    ) -> Result<Response<GetStateResponse>, Status> {
        let (command, result) = Command::get_state();
        self.controller.send_command(command);
        let response = match result.await {
            Err(_) => Err(Status::unavailable("Controller busy")),
            Ok(Err(e)) => Err(e.into()),
            Ok(Ok((position, velocity))) => {
                let response = GetStateResponse {
                    position: position.to_cm(),
                    velocity: velocity.to_cm_per_s(),
                };
                Ok(Response::new(response))
            }
        };
        info!(?request, ?response, "GetState");
        response
    }

    async fn subscribe_state(
        &self,
        request: Request<SubscribeStateRequest>,
    ) -> Result<Response<Self::SubscribeStateStream>, Status> {
        let (command, result) = Command::subscribe_state();
        self.controller.send_command(command);
        let response = match result.await {
            Err(_) => Err(Status::unavailable("Controller busy")),
            Ok(Err(e)) => Err(e.into()),
            Ok(Ok(stream)) => {
                let response_stream = stream.map(|(position, velocity)| {
                    Ok(SubscribeStateResponse {
                        position: position.to_cm(),
                        velocity: velocity.to_cm_per_s(),
                    })
                });
                Ok(Response::new(
                    Box::pin(response_stream) as Self::SubscribeStateStream
                ))
            }
        };
        info!(
            ?request,
            response = match &response {
                Ok(_) => "Ok(...)".to_owned(),
                Err(status) => format!("Err({:?})", status),
            },
            "SubscribeState",
        );
        response
    }

    async fn stop(&self, request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let (command, complete) = Command::stop();
        self.controller.send_command(command);

        let response = match complete.await {
            Err(_) => Err(Status::unavailable("Controller busy")),
            Ok(Err(e)) => Err(e.into()),
            Ok(Ok(())) => Ok(Response::new(StopResponse {})),
        };
        info!(?request, ?response, "Stop");
        response
    }

    async fn start_move(
        &self,
        request: Request<StartMoveRequest>,
    ) -> Result<Response<StartMoveResponse>, Status> {
        let target = match Position::from_cm(request.get_ref().target) {
            Ok(position) => position,
            Err(PositionError::OutOfBound(p)) => {
                return Err(Status::out_of_range(format!(
                    "{}",
                    PositionError::OutOfBound(p)
                )))
            }
            _ => unreachable!(),
        };

        let (command, complete) = Command::move_to(target);
        self.controller.send_command(command);

        let response = match complete.await {
            Err(_) => Err(Status::unavailable("Controller busy")),
            Ok(Err(e)) => Err(e.into()),
            Ok(Ok(())) => Ok(Response::new(StartMoveResponse {})),
        };
        info!(?request, ?response, "StartMove");
        response
    }
}
