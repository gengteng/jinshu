use crate::kafka::{KafkaConsumer, KafkaConsumerConfig};
use crate::pulsar::{PulsarConsumer, PulsarConsumerConfig};
use crate::QueuedMessageHandler;
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Debug, Deserialize, Serialize)]
pub enum QueueConfig<K, P> {
    #[serde(rename = "kafka")]
    Kafka(K),

    #[serde(rename = "pulsar")]
    Pulsar(P),
}

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
