use crate::pulsar::PulsarConsumerConfig;
use crate::{HandleResult, QueuedMessage, QueuedMessageHandler};
use pulsar::consumer::InitialPosition;
use pulsar::{Consumer, ConsumerOptions, Pulsar, SubType, TokioExecutor};
use std::future::Future;
use tokio_stream::StreamExt;

pub struct PulsarConsumer {
    topic: String,
    //pulsar: Pulsar<TokioExecutor>,
    consumer: Consumer<crate::QueuedMessage, TokioExecutor>,
}

impl PulsarConsumer {
    pub async fn new(config: &PulsarConsumerConfig) -> crate::pulsar::Result<Self> {
        let sub_type = match config.extension.subscription_type.as_str() {
            "exclusive" | "0" => SubType::Exclusive,
            "shared" | "1" => SubType::Shared,
            "failover" | "2" => SubType::Failover,
            "keyshared" | "3" => SubType::KeyShared,
            _ => {
                return Err(crate::pulsar::Error::InvalidSubType(
                    config.extension.subscription_type.as_str().into(),
                ))
            }
        };
        let pulsar = Pulsar::builder(config.url.to_string(), TokioExecutor)
            .build()
            .await?;
        let mut builder = pulsar
            .consumer()
            .with_topic(&config.topic)
            .with_subscription_type(sub_type);

        if let Some(consumer_name) = &config.extension.consumer_name {
            builder = builder.with_consumer_name(consumer_name);
        }

        if let Some(consumer_id) = &config.extension.consumer_id {
            builder = builder.with_consumer_id(*consumer_id);
        }

        if let Some(subscription_name) = &config.extension.subscription_name {
            builder = builder.with_subscription(subscription_name);
        }

        let consumer = builder
            .with_options(ConsumerOptions {
                initial_position: InitialPosition::Earliest,
                ..Default::default()
            })
            .build()
            .await?;

        tracing::info!(topic = %config.topic, "Topic is subscribed.");

        Ok(Self {
            topic: config.topic.clone(),
            //pulsar,
            consumer,
        })
    }

    pub async fn start_with_shutdown<F, H>(
        &mut self,
        handler: H,
        signal: F,
    ) -> crate::pulsar::Result<()>
    where
        H: QueuedMessageHandler,
        F: Future<Output = ()> + Send,
    {
        let mut shutdown = Box::pin(signal);

        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    break;
                }
                option = self.consumer.next() => {
                    if let Some(consume) = option {
                        let pulsar_message: pulsar::consumer::Message<QueuedMessage> = consume?;
                        let message = pulsar_message.deserialize()?;

                        match handler.handle(&self.topic, &message).await {
                            HandleResult::Ok => {},
                            HandleResult::Failure(error) => tracing::warn!(%error, "Failed to process message"),
                            HandleResult::Error(error) => {
                                tracing::error!(%error, "Process message error");
                                break;
                            }
                        }

                        self.consumer.ack(&pulsar_message).await?;
                    } else {
                        tracing::warn!("Consumer stream is closed.");
                        break;
                    }
                }
            }
        }

        self.consumer.unsubscribe().await?;
        tracing::info!(topic = %self.topic, "Topic is unsubscribed.");

        Ok(())
    }
}
