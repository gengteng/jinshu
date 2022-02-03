#![cfg(test)]

use crate::registry::{Change, Registry, Watcher};
use async_trait::async_trait;
use http::Uri;
use jinshu_utils::Keeper;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::broadcast::Sender;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::Stream;

type Providers = Arc<RwLock<HashMap<String, Uri>>>;
type Consumers = Arc<RwLock<HashMap<String, Sender<Change<String, Uri>>>>>;

#[derive(Clone, Default)]
pub struct MockRegistry {
    providers: Providers,
    consumers: Consumers,
}

#[async_trait::async_trait]
impl Registry for MockRegistry {
    type Error = anyhow::Error;
    type Watcher = MockWatcher;

    async fn register(
        &self,
        key: &str,
        value: &Uri,
    ) -> Result<Keeper<Result<(), Self::Error>>, Self::Error> {
        self.providers
            .write()
            .await
            .insert(key.to_string(), value.clone());

        self.consumers
            .read()
            .await
            .iter()
            .filter(|(k, _)| key.starts_with(*k))
            .for_each(|(_, v)| {
                v.send(Change::Create(key.to_string(), value.clone()))
                    .unwrap_or_default();
            });

        let registry = self.clone();
        let key = key.to_string();
        let consumers = self.consumers.clone();
        Ok(Keeper::make(|waiter| async move {
            waiter.await.unwrap_or_default();

            registry.providers.write().await.remove(&key);

            consumers
                .read()
                .await
                .iter()
                .filter(|(k, _)| key.starts_with(*k))
                .for_each(|(_, v)| {
                    v.send(Change::Delete(key.to_string())).unwrap_or_default();
                });

            Ok(())
        }))
    }

    async fn discover<T: FromIterator<(String, Uri)>>(&self, name: &str) -> Result<T, Self::Error> {
        Ok(self
            .providers
            .read()
            .await
            .iter()
            .filter(|(k, _)| k.starts_with(name))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect::<T>())
    }

    async fn watch(&self, name: &str) -> Result<Self::Watcher, Self::Error> {
        let receiver = {
            let mut lock = self.consumers.write().await;

            if let Some(sender) = lock.get(name) {
                sender.subscribe()
            } else {
                let (tx, rx) = tokio::sync::broadcast::channel(10);
                lock.insert(name.to_string(), tx);
                rx
            }
        };

        Ok(Self::Watcher {
            receiver: BroadcastStream::new(receiver),
        })
    }
}

pub struct MockWatcher {
    receiver: BroadcastStream<Change<String, Uri>>,
}

impl Stream for MockWatcher {
    type Item = Change<String, Uri>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.get_mut().receiver).poll_next(cx) {
            Poll::Ready(option) => match option {
                None => Poll::Ready(None),
                Some(Err(_)) => Poll::Ready(None),
                Some(Ok(change)) => Poll::Ready(Some(change)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

#[async_trait]
impl Watcher for MockWatcher {
    type Error = anyhow::Error;

    async fn cancel(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
