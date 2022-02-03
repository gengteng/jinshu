use crate::registry::Change;
use crate::registry::Registry;
use async_trait::async_trait;
use etcd_client::{
    Client, ConnectOptions, EventType, GetOptions, PutOptions, WatchOptions, WatchStream,
};
use http::Uri;
use jinshu_utils::Keeper;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Serialize, Deserialize)]
pub struct EtcdConfig {
    pub namespace: String,
    pub endpoints: String,
    pub user: Option<User>,
    pub keep_alive: Option<KeepAlive>,
    pub ttl: u64,
}

impl Default for EtcdConfig {
    fn default() -> Self {
        Self {
            namespace: "jinshu".into(),
            endpoints: "localhost:2379".into(),
            user: None,
            keep_alive: None,
            ttl: 10,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeepAlive {
    pub interval_ms: u64,
    pub timeout_ms: u64,
}

#[derive(Clone)]
pub struct EtcdRegistry {
    etcd: Client,
    namespace: String,
    ttl: u64,
}

impl EtcdRegistry {
    pub async fn new(config: &EtcdConfig) -> anyhow::Result<Self> {
        let mut options = ConnectOptions::new();
        if let Some(User { username, password }) = &config.user {
            options = options.with_user(username, password);
        }

        if let Some(KeepAlive {
            interval_ms,
            timeout_ms,
        }) = &config.keep_alive
        {
            options = options.with_keep_alive(
                Duration::from_millis(*interval_ms),
                Duration::from_millis(*timeout_ms),
            );
        }
        let endpoints = config.endpoints.split(',').collect::<Vec<&str>>();
        let etcd = etcd_client::Client::connect(&endpoints, Some(options)).await?;
        Ok(Self {
            etcd,
            namespace: config.namespace.clone(),
            ttl: config.ttl,
        })
    }

    pub fn get_register_key_prefix(&self, name: &str) -> String {
        format!("{}.{}.", self.namespace, name)
    }

    pub fn get_register_key(&self, name: &str, uri: &Uri) -> String {
        format!("{}.{}.{}", self.namespace, name, uri)
    }
}

#[async_trait]
impl Registry for EtcdRegistry {
    type Error = anyhow::Error;
    type Watcher = EtcdWatcher;

    async fn register(&self, name: &str, uri: &Uri) -> anyhow::Result<Keeper<anyhow::Result<()>>> {
        let ttl = self.ttl;
        let key = self.get_register_key(name, uri);

        let grant = self.etcd.lease_client().grant(ttl as i64, None).await?;
        let _ = self
            .etcd
            .kv_client()
            .put(
                key.clone(),
                uri.to_string(),
                Some(PutOptions::new().with_lease(grant.id())),
            )
            .await?;

        let (mut lease_keeper, mut stream) =
            self.etcd.lease_client().keep_alive(grant.id()).await?;

        let mut kv = self.etcd.kv_client();

        let keeper = Keeper::make(|mut waiter| async move {
            lease_keeper.keep_alive().await?;
            while let Some(result) = stream.next().await {
                let resp = result?;
                tracing::debug!(
                    "Got a keep alive response of id {} (ttl: {})",
                    resp.id(),
                    resp.ttl()
                );

                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(ttl >> 1)) => {

                    }
                    _ = &mut waiter => {
                        break;
                    }
                }

                lease_keeper.keep_alive().await?;
            }

            kv.delete(key, None).await?;
            Ok::<(), anyhow::Error>(())
        });

        Ok(keeper)
    }

    async fn discover<T: FromIterator<(String, Uri)>>(&self, name: &str) -> anyhow::Result<T> {
        let key_prefix = self.get_register_key_prefix(name);

        let value = self
            .etcd
            .kv_client()
            .get(key_prefix.clone(), Some(GetOptions::new().with_prefix()))
            .await?;

        Ok(value
            .kvs()
            .iter()
            .filter_map(|kv| match (kv.key_str(), kv.value_str()) {
                (Ok(key), Ok(value)) => match value.parse::<Uri>() {
                    Ok(uri) => Some((key.to_string(), uri)),
                    Err(error) => {
                        tracing::warn!(%error, %key, %value, "Value is an invalid uri");
                        None
                    }
                },
                (Err(error), _) | (_, Err(error)) => {
                    tracing::warn!(%error, namespace = %self.namespace, %name, "Invalid KV pair");
                    None
                }
            })
            .collect())
    }

    async fn watch(&self, name: &str) -> Result<Self::Watcher, Self::Error> {
        let key_prefix = self.get_register_key_prefix(name);
        let (watcher, stream) = self
            .etcd
            .watch_client()
            .watch(key_prefix.clone(), Some(WatchOptions::new().with_prefix()))
            .await?;

        Ok(EtcdWatcher::new(watcher, stream))
    }
}

pub struct EtcdWatcher {
    watch: etcd_client::Watcher,
    stream: WatchStream,
    cache: VecDeque<Change<String, Uri>>,
}

impl EtcdWatcher {
    pub fn new(watch: etcd_client::Watcher, stream: WatchStream) -> Self {
        Self {
            watch,
            stream,
            cache: Default::default(),
        }
    }
}

#[async_trait]
impl crate::registry::Watcher for EtcdWatcher {
    type Error = anyhow::Error;

    async fn cancel(&mut self) -> Result<(), Self::Error> {
        Ok(self.watch.cancel().await?)
    }
}

impl Stream for EtcdWatcher {
    type Item = Change<String, Uri>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let change = this.cache.pop_front();
        if change.is_some() {
            return Poll::Ready(change);
        }

        match Pin::new(&mut this.stream).poll_next(cx) {
            Poll::Ready(option) => match option {
                Some(item) => match item {
                    Ok(resp) => {
                        this.cache.extend(resp.events().iter().filter_map(
                            |event| match event.kv() {
                                Some(kv) => match event.event_type() {
                                    EventType::Put => match (kv.key_str(), kv.value_str()) {
                                        (Ok(key), Ok(value)) => match value.parse::<Uri>() {
                                            Ok(uri) => {
                                                let change = Change::Create(key.to_string(), uri);
                                                tracing::info!(?change, "Service set is changed.");
                                                Some(change)
                                            },
                                            Err(error) => {
                                                tracing::warn!(%error, %key, %value, "Value is an invalid uri");
                                                None
                                            },
                                        },
                                        _ => None,
                                    },
                                    EventType::Delete => match kv.key_str() {
                                        Ok(key)=> {
                                            let change = Change::Delete(key.to_string());
                                            tracing::info!(?change, "Service set is changed.");
                                            Some(change)
                                        },
                                        _ => None,
                                    },
                                },
                                None => None,
                            },
                        ));

                        Poll::Ready(this.cache.pop_front())
                    }
                    Err(_error) => Poll::Ready(None),
                },
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod test {
    use super::EtcdConfig;

    #[test]
    fn default() {
        EtcdConfig::default();
    }
}
