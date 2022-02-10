use crate::LoginError;
use dashmap::DashMap;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use jinshu_protocol::{
    Body, Message, Pdu, PduCodec, Request, Response, TransactionId, TransactionIdGenerator,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::codec::Framed;
use url::Url;
use uuid::Uuid;

/// Client 的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Comet 的主机名
    pub comet_host: String,
    /// Comet 的端口
    pub comet_port: u16,
    /// Api 的 URL
    pub api_url: Url,
}

impl ClientConfig {
    /// Comet 的地址
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

/// 客户端
#[derive(Debug, Clone)]
pub struct Client {
    config: ClientConfig,
    http: reqwest::Client,
}

impl Client {
    /// 使用配置构造客户端
    pub fn new(config: impl Into<ClientConfig>) -> crate::Result<Self> {
        Ok(Self {
            config: config.into(),
            http: reqwest::ClientBuilder::new()
                .user_agent(USER_AGENT)
                .build()?,
        })
    }

    /// 客户端内部使用的 HTTP 客户端
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http
    }

    /// 使用锦书用户 ID 及令牌登录
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
                let (read_sender, receiver) = tokio::sync::mpsc::channel(32);
                let w = waiting.clone();
                tokio::spawn(async move {
                    if let Err(e) = read_loop(read_sender, w, reader).await {
                        log::error!("Read loop exited with error: {}", e);
                    }
                });

                let (sender, write_receiver) = tokio::sync::mpsc::channel(32);
                tokio::spawn(async move {
                    if let Err(e) = write_loop(write_receiver, waiting, writer).await {
                        log::error!("Write loop exited with error: {}", e);
                    }
                });

                Ok(UserAgent {
                    user_id,
                    connection: Connection::new(receiver, sender),
                })
            }
            Some(Ok(Pdu {
                body: Body::Resp(Response::InvalidToken { .. }),
                ..
            })) => Err(crate::LoginError::InvalidToken),
            Some(Ok(pdu)) => Err(crate::LoginError::UnexpectedResponse(pdu)),
            Some(Err(e)) => Err(crate::LoginError::DecodeError(e)),
            None => Err(crate::LoginError::ConnectionClosed),
        }
    }
}

async fn write_loop(
    mut receiver: Receiver<Message>,
    waiting: Arc<DashMap<TransactionId, Instant>>,
    mut writer: SplitSink<Framed<TcpStream, PduCodec>, Pdu>,
) -> anyhow::Result<()> {
    let mut id_gen = TransactionIdGenerator::new();

    while let Some(message) = receiver.recv().await {
        let trans_id = id_gen.next_id();
        let pdu = Request::Send { message }.to_pdu(trans_id);

        waiting.insert(trans_id, Instant::now());

        writer.send(pdu).await?;
    }

    Ok(())
}

async fn read_loop(
    sender: Sender<Message>,
    waiting: Arc<DashMap<TransactionId, Instant>>,
    mut reader: SplitStream<Framed<TcpStream, PduCodec>>,
) -> anyhow::Result<()> {
    while let Some(qr) = reader.next().await {
        let pdu = qr?;
        match pdu.body {
            Body::Resp(response) => match waiting.remove(&pdu.id) {
                Some((_, instant)) => match response {
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
                Request::Push { message } => {
                    log::info!("Received a message: {:?}", message);
                    sender.send(message).await?;
                }
                req => log::error!("Invalid request: {:?}", req),
            },
        }
    }

    Ok(())
}

/// 客户端发送 HTTP 请求时的 User-Agent 字段
pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// 用户代理
#[derive(Debug)]
pub struct UserAgent {
    user_id: Uuid,
    connection: Connection<Message>,
}

impl UserAgent {
    /// 发送消息
    pub async fn send(&self, message: Message) -> crate::Result<()> {
        self.connection.send(message).await
    }

    /// 接收消息
    pub async fn receive(&mut self) -> crate::Result<Message> {
        self.connection.receive().await
    }

    /// 锦书用户 ID
    pub async fn user_id(&self) -> &Uuid {
        &self.user_id
    }
}

/// 用于收发 `T` 的连接
#[derive(Debug)]
pub struct Connection<T> {
    receiver: Receiver<T>,
    sender: Sender<T>,
}

impl<T> Connection<T> {
    /// 构造连接
    pub(crate) fn new(receiver: Receiver<T>, sender: Sender<T>) -> Self {
        Self { receiver, sender }
    }

    /// 发送 `T`
    pub async fn send(&self, message: T) -> crate::Result<()> {
        if self.sender.send(message).await.is_err() {
            return Err(crate::Error::ConnectionClosed);
        }
        Ok(())
    }

    /// 接收 `T`
    pub async fn receive(&mut self) -> crate::Result<T> {
        match self.receiver.recv().await {
            Some(message) => Ok(message),
            None => Err(crate::Error::ConnectionClosed),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Connection;

    #[tokio::test]
    async fn connection() {
        let (sender, receiver) = tokio::sync::mpsc::channel::<()>(1);
        let mut connection = Connection::new(receiver, sender.clone());

        assert!(connection.send(()).await.is_ok());
        assert!(connection.receive().await.is_ok());
    }
}
