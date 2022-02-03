use crate::kafka::KafkaConsumerConfig;
use crate::{HandleResult, QueuedMessage, QueuedMessageHandler};
use rdkafka::consumer::CommitMode;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use std::future::Future;
use tokio_stream::StreamExt;

pub struct KafkaConsumer {
    topic: String,
    consumer: StreamConsumer,
}

impl KafkaConsumer {
    pub async fn new(config: &KafkaConsumerConfig) -> crate::kafka::Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &config.extension.group_id)
            .set("bootstrap.servers", &config.servers)
            .set(
                "enable.partition.eof",
                config.extension.partition_eof.to_string(),
            )
            .set(
                "auto.offset.reset",
                config.extension.auto_offset_reset.to_string(),
            )
            .set(
                "session.timeout.ms",
                config.extension.session_timeout_ms.to_string(),
            )
            .set(
                "enable.auto.commit",
                config.extension.auto_commit.to_string(),
            )
            .create()?;

        consumer.subscribe(&[&config.topic])?;

        tracing::info!(topic = %config.topic, "Topic is subscribed.");

        Ok(Self {
            topic: config.topic.to_string(),
            consumer,
        })
    }

    pub async fn start_with_shutdown<F, H>(
        &mut self,
        handler: H,
        signal: F,
    ) -> crate::kafka::Result<()>
    where
        H: QueuedMessageHandler,
        F: Future<Output = ()> + Send,
    {
        struct UnsubscribeOnDrop<'a>(&'a KafkaConsumer);
        impl<'a> Drop for UnsubscribeOnDrop<'a> {
            fn drop(&mut self) {
                self.0.consumer.unsubscribe();
                tracing::info!(topic = %self.0.topic, "Topic is unsubscribed.");
            }
        }
        let _unsubscribe = UnsubscribeOnDrop(self);

        let mut stream = self.consumer.stream();

        let mut signal = Box::pin(signal);

        loop {
            tokio::select! {
                _ = &mut signal => {
                    break;
                }
                option = stream.next() => {
                    if let Some(consume) = option {
                        let kafka_message = consume?;
                        let message = QueuedMessage::try_from(&kafka_message)?;
                        match handler.handle(&self.topic, &message).await {
                            HandleResult::Ok => {},
                            HandleResult::Failure(error) => tracing::warn!(%error, "Failed to process message"),
                            HandleResult::Error(error) => {
                                tracing::error!(%error, "Process message error");
                                break;
                            }
                        }

                        self.consumer.commit_message(&kafka_message, CommitMode::Async)?;
                    } else {
                        tracing::warn!("Consumer stream is closed.");
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
