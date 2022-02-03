use crate::ConnectionManager;
use async_trait::async_trait;
use jinshu_protocol::Message;
use jinshu_rpc::comet::PushResult;
use jinshu_rpc::{internal, invalid_argument};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct Comet {
    manager: ConnectionManager,
}

impl Comet {
    pub fn new(manager: ConnectionManager) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl jinshu_rpc::comet::comet_server::Comet for Comet {
    async fn push(
        &self,
        request: Request<jinshu_rpc::domain::message::Message>,
    ) -> Result<Response<PushResult>, Status> {
        let message = request.into_inner();
        let message = Message::try_from(&message).map_err(invalid_argument)?;
        if let Some(mut r) = self.manager.get(message.to) {
            r.push(message).await.map_err(internal)?;

            Ok(Response::new(PushResult {
                ok: true,
                result: None,
            }))
        } else {
            Err(Status::not_found(format!("user {} not found.", message.to)))
        }
    }
}
