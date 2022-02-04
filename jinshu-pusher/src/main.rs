mod config;
mod pusher;

use crate::config::PusherConfig;
use crate::pusher::Pusher;
use jinshu_common::Config;
use jinshu_queue::config::{consume_with_handler, QueueConfig};
use jinshu_queue::kafka::KafkaConsumerConfig;
use jinshu_queue::pulsar::PulsarConsumerConfig;
use jinshu_redis::config::RedisConfig;
use jinshu_redis::session::SessionStore;
use jinshu_rpc::registry::etcd::{EtcdConfig, EtcdRegistry};
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Conf {
    tracing: TracingConfig,
    etcd: EtcdConfig,
    pusher: PusherConfig,
    redis: RedisConfig,

    /// 使用的消息队列中间件
    #[serde(flatten)]
    queue: QueueConfig<KafkaConsumerConfig, PulsarConsumerConfig>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracing = conf.tracing.init("pusher");

    tracing::info!(?conf);

    let Conf {
        etcd,
        redis,
        pusher,
        queue,
        ..
    } = conf;

    let etcd = EtcdRegistry::new(&etcd).await?;
    let session_store = SessionStore::new(redis).await?;

    let pusher = Pusher::new(&pusher.comet_name, &etcd, session_store).await?;

    consume_with_handler(queue, pusher, shutdown_signal()).await?;

    Ok(())
}
