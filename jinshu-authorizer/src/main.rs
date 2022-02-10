use jinshu_authorizer::authorizer::Authorizer;
use jinshu_authorizer::config::AuthorizerConfig;
use jinshu_common::Config;
use jinshu_redis::config::RedisConfig;
use jinshu_rpc::authorizer::authorizer_server::AuthorizerServer;
use jinshu_rpc::registry::etcd::{EtcdConfig, EtcdRegistry};
use jinshu_rpc::registry::Registry;
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Conf {
    tracing: TracingConfig,
    etcd: EtcdConfig,
    redis: RedisConfig,
    authorizer: AuthorizerConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracer = conf.tracing.init("authorizer")?;

    let Conf {
        etcd,
        redis,
        authorizer: AuthorizerConfig { service },
        ..
    } = conf;

    tracing::info!(?etcd);
    let registry = EtcdRegistry::new(&etcd).await?;

    tracing::info!(?redis);
    let redis_config: deadpool_redis::Config = redis.into();
    let redis = redis_config.builder()?.build()?;

    let authorizer = AuthorizerServer::new(Authorizer::new(redis));

    let (uri, handle) = registry
        .run_service(service, authorizer, shutdown_signal())
        .await?;

    tracing::info!(%uri, "Authorizer service is running.");
    handle.await?;

    Ok(())
}
