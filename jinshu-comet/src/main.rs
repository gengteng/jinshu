use jinshu_comet::comet::Comet;
use jinshu_comet::config::CometConfig;
use jinshu_comet::connection::ConnectionManager;
use jinshu_common::Config;
use jinshu_redis::config::RedisConfig;
use jinshu_redis::session::SessionStore;
use jinshu_rpc::authorizer::authorizer_client;
use jinshu_rpc::comet::comet_server::CometServer;
use jinshu_rpc::receiver::receiver_client;
use jinshu_rpc::registry::etcd::{EtcdConfig, EtcdRegistry};
use jinshu_rpc::registry::Registry;
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Debug, Deserialize)]
struct Conf {
    comet: CometConfig,
    tracing: TracingConfig,
    etcd: EtcdConfig,
    redis: RedisConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracer = conf.tracing.init("comet")?;

    let Conf {
        etcd,
        redis,
        comet:
            CometConfig {
                ip,
                port,
                codec,
                service,
                receiver_name,
                authorizer_name,
            },
        ..
    } = conf;

    tracing::info!(?etcd);
    let registry = EtcdRegistry::new(&etcd).await?;

    let (receiver_channel, rk) = registry.discover_channel(&receiver_name).await?;
    let receiver = receiver_client::ReceiverClient::new(receiver_channel);

    let (authorizer_channel, ak) = registry.discover_channel(&authorizer_name).await?;
    let authorizer = authorizer_client::AuthorizerClient::new(authorizer_channel);

    let socket_addr = SocketAddr::new(ip, port);
    let server = TcpListener::bind(socket_addr).await?;

    tracing::info!(%socket_addr, "jinshu-comet started.");

    let (listener, service_uri) = service.try_bind().await?;

    tracing::info!(?redis);
    let redis_config: deadpool_redis::Config = redis.into();
    let redis = redis_config.builder()?.build()?;
    let session_store = SessionStore::from_pool(redis);

    let register_key = registry.get_register_key(&service.service_name, &service_uri);
    let connection_manager =
        ConnectionManager::new(&register_key, receiver, authorizer, session_store);

    let comet = Comet::new(connection_manager.clone());
    let mut handle = registry
        .run_service_with_listener(
            &service.service_name,
            &service_uri,
            listener,
            CometServer::new(comet),
            shutdown_signal(),
        )
        .await?;

    tracing::info!(uri = %service_uri, "Comet service is running.");

    loop {
        tokio::select! {
            result = server.accept() => {
                match result {
                    Ok((socket, addr)) => {
                        tracing::info!("[TCP] {} => {} connected", addr, socket_addr);
                        let mut cm = connection_manager.clone();
                        tokio::spawn(async move {
                            if let Err(e) = cm.accept(socket, codec).await {
                                tracing::error!("Failed to accept user connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("[TCP] accept error: {}", e);
                        break;
                    }
                }
            }
            _ = &mut handle => {
                break;
            }
        }
    }

    rk.close().await??;
    ak.close().await??;
    tracing::info!("Service keeper closed.");

    Ok(())
}
