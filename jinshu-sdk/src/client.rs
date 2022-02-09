use crate::LoginError;
use dashmap::DashMap;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use jinshu_protocol::{
    Body, Codec, Content, Message, Pdu, PduCodec, Request, Response, TransactionId,
    TransactionIdGenerator,
};
use jinshu_utils::current_millisecond;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::codec::Framed;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub comet_host: String,
    pub comet_port: u16,
    pub api_url: Url,
}

impl ClientConfig {
    pub fn comet_address(&self) -> String {
        format!("{}:{}", self.comet_host, self.comet_port)
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            comet_host: "localhost".into(),
            comet_port: 9000,
            api_url: "http://localhost:9500"
                .parse()
                .expect("impossible: api_url parse error"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    config: ClientConfig,
    http: reqwest::Client,
}

impl Client {
    pub fn new(config: impl Into<ClientConfig>) -> crate::Result<Self> {
        Ok(Self {
            config: config.into(),
            http: reqwest::ClientBuilder::new()
                .user_agent(USER_AGENT)
                .build()?,
        })
    }

    pub async fn sign_in(&self, user_id: Uuid, token: Uuid) -> Result<UserAgent, LoginError> {
        let socket = TcpStream::connect(self.config.comet_address()).await?;
        let mut framed = Framed::new(socket, PduCodec::default());
        let mut trans_id_gen = TransactionIdGenerator::default();

        let sign_in = Request::SignIn { user_id, token }.to_pdu(trans_id_gen.next_id());

        framed.send(sign_in).await?;

        // add timeout
        match framed.next().await {
            Some(Ok(Pdu {
                body: Body::Resp(Response::SignedIn { extension }),
                ..
            })) => {
                log::info!("Sign in ok");
                if let Some(extension) = &extension {
                    log::info!("extension: {}", extension);
                }
                // TODO: extension
                let (writer, reader) = framed.split();

                let waiting = Arc::new(DashMap::new());

                Ok(UserAgent {
                    user_id,
                    token,
                    connection: UserConnection::new(trans_id_gen, framed),
                })
            }
            Some(Ok(Pdu {
                body: Body::Resp(Response::InvalidToken { user_id }),
                ..
            })) => Err(crate::LoginError::InvalidToken.into()),
            Some(Ok(pdu)) => Err(crate::LoginError::UnexpectedResponse(pdu).into()),
            Some(Err(e)) => Err(crate::LoginError::DecodeError(e)),
            None => Err(crate::LoginError::ConnectionClosed.into()),
        }
    }
}

async fn write_loop(
    waiting: Arc<DashMap<TransactionId, Instant>>,
    mut writer: SplitSink<Framed<TcpStream, PduCodec>, Pdu>,
) -> anyhow::Result<()> {
    let mut id_gen = TransactionIdGenerator::new();

    loop {
        let trans_id = id_gen.next_id();

        let pdu = Request::Send {
            message: Message {
                id: Uuid::new_v4(),
                timestamp: current_millisecond(),
                from,
                to,
                content: Content::Data {
                    mime: mime::TEXT_PLAIN_UTF_8,
                    bytes: Vec::from("你好"),
                },
            },
        }
        .to_pdu(trans_id);

        waiting.lock().await.insert(trans_id, Instant::now());

        writer.send(pdu).await?;

        sleep(Duration::from_secs(1)).await;
    }
}

async fn read_loop(
    waiting: Arc<Mutex<HashMap<TransactionId, Instant>>>,
    mut reader: SplitStream<Framed<TcpStream, PduCodec>>,
) -> anyhow::Result<()> {
    while let Some(qr) = reader.next().await {
        let pdu = qr?;
        match pdu.body {
            Body::Resp(response) => match waiting.lock().await.remove(&pdu.id) {
                Some(instant) => match response {
                    Response::Queued { id } => {
                        log::info!(
                            "Message {:?} is queued. ({}ms)",
                            id,
                            instant.elapsed().as_millis()
                        );
                    }
                    Response::Rejected { id, error } => {
                        log::error!(
                            "Message {:?} is rejected: {}. ({}ms)",
                            id,
                            error,
                            instant.elapsed().as_millis()
                        )
                    }
                    resp => log::error!("Invalid response: {:?}", resp),
                },
                None => {
                    log::error!(
                        "Invalid transaction id: {:?}, waiting: {:?}",
                        pdu.id,
                        waiting
                    );
                    break;
                }
            },
            Body::Req(request) => match request {
                Request::Push { message } => log::info!("Received a message: {:?}", message),
                req => log::error!("Invalid request: {:?}", req),
            },
        }
    }

    Ok(())
}

pub const USER_AGENT: &'static str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct UserAgent<T = TcpStream> {
    user_id: Uuid,
    token: Uuid,
    connection: UserConnection<T>,
}

impl UserAgent {
    pub async fn send(&self, _message: Message) -> crate::Result<()> {
        Ok(())
    }

    pub async fn receive(&self) -> crate::Result<Message> {
        Err(crate::Error::Other("todo".into()))
    }

    pub async fn sign_out(self) -> crate::Result<()> {
        Err(crate::Error::Other("todo".into()))
    }
}

#[derive(Debug)]
pub struct UserConnection<T> {
    trans_id_gen: TransactionIdGenerator,
    framed: Framed<T, PduCodec>,
}

impl<T> UserConnection<T> {
    pub(crate) fn new(trans_id_gen: TransactionIdGenerator, framed: Framed<T, PduCodec>) -> Self
    where
        T: AsyncWrite + AsyncRead,
    {
        Self {
            trans_id_gen,
            framed,
        }
    }
}
