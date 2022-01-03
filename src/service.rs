use crate::{
    controllers::{Command, CommandSender, CommandSenderExt, ControllerError},
    logging,
    utils::{Position, PositionError, Velocity},
};
use async_trait::async_trait;
pub use desk_service::desk_service_server::DeskServiceServer;
use desk_service::{
    desk_service_server::DeskService as DeskServiceTrait, GetStateRequest, GetStateResponse,
    StartMoveRequest, StartMoveResponse, State, StopRequest, StopResponse, SubscribeStateRequest,
    SubscribeStateResponse,
};
use futures::{Stream, StreamExt};
use std::{convert::Infallible, pin::Pin};
use tonic::{Request, Response, Status};

mod desk_service {
    tonic::include_proto!("desk_service");
}

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

impl From<(Position, Velocity)> for State {
    fn from(state: (Position, Velocity)) -> State {
        let (position, velocity) = state;
        State {
            position: desk_service::Position {
                value: position.to_cm(),
            }
            .into(),
            velocity: desk_service::Velocity {
                value: velocity.to_cm_per_s(),
            }
            .into(),
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
            Ok(Ok(state)) => {
                let response = GetStateResponse {
                    state: Some(state.into()),
                };
                Ok(Response::new(response))
            }
        };
        logging::info!("Received GetState request"; "request" => ?request, "response" => ?response);
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
                let response_stream = stream.map(|state| {
                    Ok(SubscribeStateResponse {
                        state: Some(state.into()),
                    })
                });
                Ok(Response::new(
                    Box::pin(response_stream) as Self::SubscribeStateStream
                ))
            }
        };
        logging::info!(
            "Received GetState request";
            "request" => ?request,
            "response" => match &response {
                Ok(_) => "Ok(...)".to_owned(),
                Err(status) => format!("{:?}", Result::<Infallible, _>::Err(status))
            }
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
        logging::info!("Received Stop request"; "request" => ?request, "response" => ?response);
        response
    }

    async fn start_move(
        &self,
        request: Request<StartMoveRequest>,
    ) -> Result<Response<StartMoveResponse>, Status> {
        logging::info!("Received StartMove request"; "request" => ?request);
        let target = request.get_ref().target.as_ref().ok_or(()).or_else(|()| {
            let response = Err(Status::invalid_argument("No target position"));
            logging::info!("Received StartMove request"; "request" => ?request, "response" => ?response);
            response
        })?;

        let target = match Position::from_cm(target.value) {
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
        logging::info!("Received StartMove request"; "request" => ?request, "response" => ?response);
        response
    }
}
