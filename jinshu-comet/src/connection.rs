use dashmap::mapref::one::RefMut;
use dashmap::DashMap;
use futures::SinkExt;
use jinshu_protocol::{
    Body, Codec, Message, Pdu, PduCodec, Request, Response, TransactionIdGenerator,
};
use jinshu_redis::session::SessionStore;
use jinshu_rpc::authorizer::authorizer_client::AuthorizerClient;
use jinshu_rpc::authorizer::{SignInResult, Token};
use jinshu_rpc::domain::message::Message as RpcMessage;
use jinshu_rpc::receiver::receiver_client::ReceiverClient;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Sender};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};
use tonic::transport::Channel;
use uuid::Uuid;

#[derive(Clone)]
pub struct ConnectionManager {
    service_uri: String,
    connections: Arc<DashMap<Uuid, Connection>>,
    receiver: ReceiverClient<Channel>,
    authorizer: AuthorizerClient<Channel>,
    session_store: SessionStore,
}

impl ConnectionManager {
    pub fn new(
        service_uri: &str,
        receiver: ReceiverClient<Channel>,
        authorizer: AuthorizerClient<Channel>,
        session_store: SessionStore,
    ) -> Self {
        Self {
            service_uri: service_uri.to_owned(),
            connections: Default::default(),
            receiver,
            authorizer,
            session_store,
        }
    }

    pub async fn accept(&mut self, stream: TcpStream, codec: Codec) -> anyhow::Result<()> {
        let codec = PduCodec::new(codec);
        let (reader, writer) = stream.into_split();

        let mut writer = FramedWrite::new(writer, codec);
        let mut reader = FramedRead::new(reader, codec);

        let user_id = match reader.next().await {
            Some(Ok(Pdu {
                body: Body::Req(Request::SignIn { user_id, token }),
                id,
            })) => {
                let request = tonic::Request::new(Token {
                    user_id: user_id.simple().to_string(),
                    token: token.simple().to_string(),
                });
                match self.authorizer.sign_in(request).await {
                    Ok(resp) => {
                        let SignInResult { ok, extension } = resp.into_inner();
                        if ok {
                            writer
                                .send(
                                    Response::SignedIn {
                                        extension: extension
                                            .map(|s| serde_json::Value::from_str(&s).ok())
                                            .flatten(),
                                    }
                                    .to_pdu(id),
                                )
                                .await?;
                            user_id // get user ID
                        } else {
                            writer
                                .send(Response::InvalidToken { user_id }.to_pdu(id))
                                .await
                                .unwrap_or_default(); // do nothing
                            anyhow::bail!("Sign in error: invalid token");
                        }
                    }
                    Err(e) => {
                        writer
                            .send(
                                Response::Error {
                                    cause: e.to_string(),
                                }
                                .to_pdu(id),
                            )
                            .await
                            .unwrap_or_default(); // do nothing
                        anyhow::bail!("Sign in error: {}", e);
                    }
                }
            }
            Some(Ok(Pdu { id, body })) => {
                writer
                    .send(
                        Response::Error {
                            cause: "Sign-in request expected".to_string(),
                        }
                        .to_pdu(id),
                    )
                    .await
                    .unwrap_or_default(); // do nothing
                anyhow::bail!("Sign in error: unexpected request: {:?}", body);
            }
            Some(Err(e)) => anyhow::bail!("Read error: {}", e),
            None => anyhow::bail!("Connection closed"),
        };

        tracing::info!(%user_id, "user sign in [OK]");

        let (client_writer, mut transfer) = channel::<Pdu>(32);
        tokio::spawn(async move {
            while let Some(result) = transfer.recv().await {
                match writer.send(result).await {
                    Ok(_) => {}
                    Err(e) => tracing::error!("Failed to send pdu to client: {}", e),
                }
            }
        });

        let (transfer, mut client_reader) = channel::<Pdu>(32);
        tokio::spawn(async move {
            while let Some(result) = reader.next().await {
                match result {
                    Ok(pdu) => {
                        if let Err(e) = transfer.send(pdu).await {
                            tracing::error!(
                                "Connection closed, failed to receive a pdu: {:?}",
                                e.0
                            );
                        }
                    }
                    Err(e) => tracing::error!("Failed to read the pdu from client: {}", e),
                }
            }
        });

        let pusher = client_writer.clone();
        self.connections
            .insert(user_id, Connection::create(user_id, pusher));
        self.session_store.store(user_id, &self.service_uri).await?;

        let ss = self.session_store.clone();
        let mut receiver = self.receiver.clone();
        let connections = self.connections.clone();
        tokio::spawn(async move {
            while let Some(pdu) = client_reader.recv().await {
                tracing::info!("receive pdu: {:?}", pdu);
                let Pdu { id: req_id, body } = pdu;
                if let Body::Req(req) = body {
                    match req {
                        Request::Ping => {
                            if let Err(e) = client_writer.send(Response::Pong.to_pdu(req_id)).await
                            {
                                tracing::error!("Failed to send pong to client: {:?}", e.0);
                                break;
                            }
                        }
                        Request::Send { message } => {
                            let rpc_message = RpcMessage::try_from(&message)?;
                            let req = tonic::Request::new(rpc_message);
                            match receiver.enqueue(req).await {
                                Ok(resp) => {
                                    let result = resp.into_inner();
                                    tracing::info!("enqueue result: {}", result.ok);

                                    if let Err(e) = client_writer
                                        .send(Response::Queued { id: message.id }.to_pdu(req_id))
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to send response to client: {:?}",
                                            e.0
                                        );
                                        break;
                                    }
                                }
                                Err(e) => {
                                    if let Err(e) = client_writer
                                        .send(
                                            Response::Rejected {
                                                id: message.id,
                                                error: e.to_string(),
                                            }
                                            .to_pdu(req_id),
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to send response to client: {:?}",
                                            e.0
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                        n => {
                            tracing::error!("unexpected request: {:?}", n);
                            break;
                        }
                    }
                }
            }

            if let Err(error) = ss.remove(user_id).await {
                tracing::warn!(%error, "Failed to remove session");
            }

            match connections.remove(&user_id) {
                None => tracing::warn!(%user_id, "Failed to remove connection"),
                Some(_) => tracing::info!(%user_id, "User connection removed"),
            }

            Ok::<_, anyhow::Error>(())
        });

        Ok(())
    }

    pub fn get(&self, user_id: Uuid) -> Option<RefMut<Uuid, Connection>> {
        self.connections.get_mut(&user_id)
    }

    #[allow(dead_code)]
    pub fn remove(&self, user_id: Uuid) -> Option<(Uuid, Connection)> {
        self.connections.remove(&user_id)
    }
}

pub struct Connection {
    user_id: Uuid,
    pusher: Sender<Pdu>,
    id_gen: TransactionIdGenerator,
}

impl Connection {
    fn create(user_id: Uuid, pusher: Sender<Pdu>) -> Self {
        Self {
            user_id,
            pusher,
            id_gen: TransactionIdGenerator::default(),
        }
    }

    #[allow(dead_code)]
    pub fn user_id(&self) -> &Uuid {
        &self.user_id
    }

    #[allow(dead_code)]
    pub async fn push(&mut self, message: Message) -> anyhow::Result<()> {
        let id = self.id_gen.next_id();
        Ok(self
            .pusher
            .send(Request::Push { message }.to_pdu(id))
            .await
            .map_err(|e| {
                anyhow::anyhow!("Connection closed, Failed to send pdu to client: {:?}", e.0)
            })?)
    }
}
