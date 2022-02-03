mod config;
mod pusher;

use crate::config::PusherConfig;
use crate::pusher::Pusher;
use jinshu_common::Config;
use jinshu_queue::config::QueueConfig;
use jinshu_queue::kafka::{KafkaConsumer, KafkaConsumerConfig};
use jinshu_queue::pulsar::{PulsarConsumer, PulsarConsumerConfig};
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

    match queue {
        QueueConfig::Kafka(config) => {
            let mut kafka_consumer = KafkaConsumer::new(&config).await?;
            kafka_consumer
                .start_with_shutdown(pusher, shutdown_signal())
                .await?;
        }
        QueueConfig::Pulsar(config) => {
            let mut pulsar_consumer = PulsarConsumer::new(&config).await?;
            pulsar_consumer
                .start_with_shutdown(pusher, shutdown_signal())
                .await?;
        }
    }

    Ok(())
}
