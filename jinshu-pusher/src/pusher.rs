use dashmap::DashMap;
use jinshu_queue::{HandleResult, QueuedMessage, QueuedMessageHandler};
use jinshu_redis::session::SessionStore;
use jinshu_rpc::comet::comet_client::CometClient;
use jinshu_rpc::domain::message::Message as RpcMessage;
use jinshu_rpc::registry::etcd::EtcdRegistry;
use jinshu_rpc::registry::{Change, Registry, Watcher};
use jinshu_utils::Keeper;
use tokio_stream::StreamExt;
use tonic::transport::{Channel, Endpoint, Uri};
use tonic::Request;
use uuid::Uuid;

/// 消息推送器
pub struct Pusher {
    clients: DashMap<String, CometClient<Channel>>,
    session_store: SessionStore,
    _keeper: Keeper<anyhow::Result<()>>,
}

impl Pusher {
    /// 构造消息推送器
    pub async fn new(
        comet_name: &str,
        registry: &EtcdRegistry,
        session_store: SessionStore,
    ) -> anyhow::Result<Self> {
        let clients = DashMap::new();

        let mut watcher = registry.watch(comet_name).await?;

        let endpoints = registry.discover::<Vec<(String, Uri)>>(comet_name).await?;
        tracing::info!(?endpoints, "Endpoints are discovered");

        for (key, uri) in endpoints {
            let endpoint = Endpoint::from(uri.clone());
            match endpoint.connect().await {
                Ok(channel) => {
                    clients.insert(key, CometClient::new(channel));
                }
                Err(error) => {
                    tracing::warn!(%error, uri = %endpoint.uri(), "Failed to connect to endpoint");
                }
            }
        }

        let c = clients.clone();
        let keeper = Keeper::make(|mut waiter| async move {
            loop {
                tokio::select! {
                    _ = &mut waiter => {
                        break;
                    }
                    option = watcher.next() => {
                        match option {
                            Some(change) => {
                                tracing::info!(?change, "Service set has changed");
                                match change {
                                    Change::Create(key, uri) => {
                                        let endpoint = Endpoint::from(uri.clone());
                                        match endpoint.connect().await {
                                            Ok(channel) => {
                                                c.insert(key, CometClient::new(channel));
                                            }
                                            Err(error) => {
                                                tracing::error!(%error, uri = %endpoint.uri(), "Failed to connect to endpoint");
                                            }
                                        }
                                    }
                                    Change::Delete(key) => {
                                        c.remove(&key);
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                }
            }

            watcher.cancel().await?;
            Ok::<(), anyhow::Error>(())
        });

        Ok(Self {
            clients,
            session_store,
            _keeper: keeper,
        })
    }

    /// 发送消息
    pub async fn send(&self, message: RpcMessage) -> anyhow::Result<()> {
        let user_id: Uuid = Uuid::from_slice(message.to.as_slice())?;
        match self.session_store.load(user_id).await? {
            Some(uri) => {
                if let Some(mut client) = self.clients.get_mut(&uri) {
                    client.push(Request::new(message)).await?;
                    Ok(())
                } else {
                    tracing::info!(%uri, "Endpoint is offline");
                    anyhow::bail!("Endpoint is offline")
                }
            }
            None => {
                tracing::info!(%user_id, "User is offline");
                anyhow::bail!("User is offline")
            }
        }
    }
}

#[async_trait::async_trait]
impl QueuedMessageHandler for Pusher {
    async fn handle(&self, topic: &str, message: &QueuedMessage) -> HandleResult {
        let message: RpcMessage = message.inner().clone();
        tracing::info!(%topic, "Push message");
        if let Err(e) = self.send(message).await {
            HandleResult::Failure(e.to_string().into())
        } else {
            HandleResult::Ok
        }
    }
}
