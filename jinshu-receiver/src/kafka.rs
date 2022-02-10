use jinshu_queue::kafka::KafkaProducerConfig;
use jinshu_queue::QueuedMessage;
use jinshu_rpc::domain;
use jinshu_rpc::receiver::{self, receiver_server};
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

/// Kafka 接收器
#[derive(Clone)]
pub struct KafkaReceiver(FutureProducer, String);

// .\kafka-topics.sh --zookeeper localhost:2181 --create --topic jinshu.test --partitions 32 --replication-factor 1

impl KafkaReceiver {
    /// 使用 Kafka 的消费者配置构造
    pub fn create(config: KafkaProducerConfig) -> anyhow::Result<Self> {
        let cli = rdkafka::ClientConfig::new()
            .set("bootstrap.servers", config.servers)
            .set(
                "message.timeout.ms",
                config.extension.message_timeout.to_string(),
            )
            .create()?;
        Ok(Self(cli, config.topic))
    }
}

#[tonic::async_trait]
impl receiver_server::Receiver for KafkaReceiver {
    async fn enqueue(
        &self,
        request: tonic::Request<domain::message::Message>,
    ) -> Result<tonic::Response<receiver::EnqueueResult>, tonic::Status> {
        let message = request.into_inner();
        let message = QueuedMessage::new(message);
        match self
            .0
            .send(
                FutureRecord::to(&self.1)
                    .key(message.inner().id.as_slice())
                    .payload(&Vec::<u8>::from(&message)),
                Duration::from_secs(0),
            )
            .await
        {
            Ok((partition, offset)) => {
                tracing::info!("enqueue ok: {}, {}", partition, offset)
            }
            Err((e, _msg)) => {
                tracing::error!("error: {}", e);
                return Err(tonic::Status::new(tonic::Code::Internal, e.to_string()));
            }
        }
        Ok(tonic::Response::new(receiver::EnqueueResult {
            ok: true,
            result: None,
        }))
    }
}
