/// Etcd 注册中心
pub mod etcd;
/// Mock 注册中心
#[cfg(test)]
pub mod mock;

use crate::config::ServiceConfig;
use async_trait::async_trait;
use http::Uri;
use jinshu_utils::Keeper;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_stream::{Stream, StreamExt};
use tonic::body::BoxBody;
use tonic::codegen::http::{Request, Response};
use tonic::transport::{Body, Channel, Endpoint, NamedService, Server};
use tower::Service;

/// 服务注册中心
///
#[async_trait]
pub trait Registry {
    /// 注册中心错误类型
    type Error: Debug
        + Display
        + Send
        + 'static
        + From<<<Self as Registry>::Watcher as Watcher>::Error>;
    /// 注册中心监听器类型
    type Watcher: Watcher + Unpin + Send + 'static;

    /// 注册服务
    ///
    async fn register(
        &self,
        key: &str,
        value: &Uri,
    ) -> Result<Keeper<Result<(), Self::Error>>, Self::Error>;

    /// 发现服务
    ///
    async fn discover<T: FromIterator<(String, Uri)>>(&self, name: &str) -> Result<T, Self::Error>;

    /// 监听服务
    ///
    async fn watch(&self, name: &str) -> Result<Self::Watcher, Self::Error>;

    /// 发现服务并持续监听变化
    ///
    async fn discover_channel(
        &self,
        name: &str,
    ) -> Result<(Channel, Keeper<Result<(), Self::Error>>), Self::Error> {
        let mut watcher = self.watch(name).await?;

        let (channel, sender) = Channel::balance_channel(10);

        let endpoints = self.discover::<Vec<(String, Uri)>>(name).await?;
        tracing::info!(?endpoints, "Endpoints are discovered");

        for (key, uri) in endpoints {
            sender
                .send(tower::discover::Change::Insert(key, Endpoint::from(uri)))
                .await
                .unwrap_or_default()
        }

        let keeper = Keeper::make(|mut waiter| async move {
            loop {
                tokio::select! {
                    _ = &mut waiter => {
                        break;
                    }
                    option = watcher.next() => {
                        match option {
                            Some(change) => {
                                if sender.send(change.into()).await.is_err() {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                }
            }

            watcher.cancel().await?;
            Ok::<(), Self::Error>(())
        });

        Ok((channel, keeper))
    }

    /// 注册服务并保持，`signal` 完成时停止
    ///
    async fn register_with_shutdown<F: Future<Output = ()> + Send + 'static>(
        &self,
        name: &str,
        uri: &Uri,
        signal: F,
    ) -> Result<JoinHandle<()>, Self::Error> {
        let keeper = self.register(name, uri).await?;
        tracing::info!(%name, %uri, "Service is registered.");

        let name = name.to_string();
        let handle = tokio::spawn(async move {
            signal.await;

            let gracefully = keeper.close().await.is_ok();
            tracing::info!(%name, %gracefully, "Service is unregistered.");
        });

        Ok(handle)
    }

    /// 运行服务并注册，`signal` 完成时停止
    ///
    async fn run_service<S, F>(
        &self,
        config: ServiceConfig,
        service: S,
        signal: F,
    ) -> anyhow::Result<(Uri, JoinHandle<()>)>
    where
        S: Service<Request<Body>, Response = Response<BoxBody>>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
        F: Future<Output = ()> + Send + 'static,
    {
        let (listener, service_uri) = config.try_bind().await?;

        let handle = self
            .run_service_with_listener(
                &config.service_name,
                &service_uri,
                listener,
                service,
                signal,
            )
            .await?;

        Ok((service_uri, handle))
    }

    /// 使用已有的 TCP 监听器运行服务
    async fn run_service_with_listener<S, F>(
        &self,
        service_name: &str,
        service_uri: &Uri,
        listener: TcpListener,
        service: S,
        signal: F,
    ) -> anyhow::Result<JoinHandle<()>>
    where
        S: Service<Request<Body>, Response = Response<BoxBody>>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
        F: Future<Output = ()> + Send + 'static,
    {
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

        let register = match self
            .register_with_shutdown(service_name, service_uri, signal)
            .await
        {
            Ok(register) => register,
            Err(error) => {
                anyhow::bail!(
                    "Failed to register service {} ({}): {}",
                    service_name,
                    service_uri,
                    error
                );
            }
        };

        let layer = tower::ServiceBuilder::new()
            .timeout(Duration::from_secs(5))
            .layer(tower_http::trace::TraceLayer::new_for_grpc());

        let handle = tokio::spawn(async move {
            if let Err(error) = Server::builder()
                .layer(layer)
                .add_service(service)
                .serve_with_incoming_shutdown(incoming, async move {
                    if let Err(error) = register.await {
                        tracing::error!(%error, "The service is shutdown due to some error")
                    }
                })
                .await
            {
                tracing::error!(%error, "Axum server error")
            }
        });

        Ok(handle)
    }
}

/// 监听器
#[async_trait]
pub trait Watcher: Stream<Item = Change<String, Uri>> {
    /// 监听器错误类型
    type Error: Debug + Display + Send + Sync + 'static;

    /// 取消监听
    async fn cancel(&mut self) -> Result<(), Self::Error>;
}

/// 负载变化
#[derive(Debug, Clone)]
pub enum Change<K, V> {
    /// 创建
    Create(
        /// 键
        K,
        /// 值
        V,
    ),
    /// 删除
    Delete(
        /// 键
        K,
    ),
}

impl<K> From<Change<K, Uri>> for tower::discover::Change<K, Endpoint> {
    fn from(change: Change<K, Uri>) -> Self {
        match change {
            Change::Create(k, v) => tower::discover::Change::Insert(k, Endpoint::from(v)),
            Change::Delete(k) => tower::discover::Change::Remove(k),
        }
    }
}

#[cfg(test)]
mod test {
    use super::mock::MockRegistry;
    use crate::config::ServiceConfig;
    use crate::registry::Registry;
    use crate::test::{Ping, Pong};
    use async_trait::async_trait;
    use futures::FutureExt;
    use rand::Rng;
    use std::net::IpAddr;
    use tonic::{Request, Response, Status};
    use uuid::Uuid;

    #[async_trait]
    impl crate::test::test_server::Test for i32 {
        async fn test(&self, request: Request<Ping>) -> Result<Response<Pong>, Status> {
            let ping = request.into_inner();
            Ok(Response::new(Pong {
                ack: ping.syn.wrapping_add(*self),
            }))
        }
    }

    #[tokio::test]
    async fn registry() -> anyhow::Result<()> {
        let registry = MockRegistry::default();

        let service = rand::thread_rng().gen_range(1..4);
        let service_name = Uuid::new_v4().to_string();
        let config = ServiceConfig {
            service_name: service_name.clone(),
            public_host: "0.0.0.0".into(),
            listen_ip: IpAddr::from([0, 0, 0, 0]),
            listen_port: 0,
        };

        let (closer, waiter) = tokio::sync::oneshot::channel::<()>();

        let r = registry.clone();
        let _ = r
            .run_service(
                config,
                crate::test::test_server::TestServer::new(service),
                waiter.map(|_| ()),
            )
            .await;

        let (channel, keeper) = registry.discover_channel(&service_name).await?;

        let mut client = crate::test::test_client::TestClient::new(channel);

        let syn = rand::random();
        let resp = client.test(Request::new(Ping { syn })).await?;
        let pong = resp.into_inner();
        assert_eq!(syn.wrapping_add(service), pong.ack);

        closer.send(()).unwrap_or_default();
        keeper.close().await??;

        Ok(())
    }
}
