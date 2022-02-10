use crate::kafka::{KafkaConsumer, KafkaConsumerConfig};
use crate::pulsar::{PulsarConsumer, PulsarConsumerConfig};
use crate::QueuedMessageHandler;
use serde::{Deserialize, Serialize};
use std::future::Future;

/// 队列配置
#[derive(Debug, Deserialize, Serialize)]
pub enum QueueConfig<K, P> {
    /// Kafka 配置
    #[serde(rename = "kafka")]
    Kafka(
        /// 配置值
        K,
    ),

    /// Pulsar 配置
    #[serde(rename = "pulsar")]
    Pulsar(
        /// 配置值
        P,
    ),
}

/// 使用给定的消息处理器消费队列
///
/// 使用消费者配置构造消费者对象，对消费的每个消息调用处理器，在 `signal` 触发时退出
///
pub async fn consume_with_handler<
    H: QueuedMessageHandler,
    F: Future<Output = ()> + Send + 'static,
>(
    consumer_config: QueueConfig<KafkaConsumerConfig, PulsarConsumerConfig>,
    handler: H,
    signal: F,
) -> crate::error::Result<()> {
    match consumer_config {
        QueueConfig::Kafka(config) => {
            let mut kafka_consumer = KafkaConsumer::new(&config).await?;
            kafka_consumer.start_with_shutdown(handler, signal).await?;
        }
        QueueConfig::Pulsar(config) => {
            let mut pulsar_consumer = PulsarConsumer::new(&config).await?;
            pulsar_consumer.start_with_shutdown(handler, signal).await?;
        }
    }

    Ok(())
}
