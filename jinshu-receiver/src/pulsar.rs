use jinshu_queue::pulsar::PulsarProducerConfig;
use jinshu_queue::QueuedMessage;
use jinshu_rpc::domain::message::Message;
use jinshu_rpc::receiver::receiver_server::Receiver;
use jinshu_rpc::receiver::EnqueueResult;
use pulsar::producer::Message as PulsarMessage;
use pulsar::{Pulsar, TokioExecutor};
use tonic::{Request, Response, Status};

/// Pulsar 接收器
pub struct PulsarReceiver(Pulsar<TokioExecutor>, PulsarProducerConfig);

impl PulsarReceiver {
    /// 使用 Pulsar 消费者配置构造
    pub async fn create(config: PulsarProducerConfig) -> anyhow::Result<Self> {
        let pulsar = Pulsar::builder(config.url.to_string(), TokioExecutor)
            .build()
            .await?;
        //let producer = pulsar.producer();
        Ok(Self(pulsar, config))
    }
}

#[tonic::async_trait]
impl Receiver for PulsarReceiver {
    async fn enqueue(&self, request: Request<Message>) -> Result<Response<EnqueueResult>, Status> {
        let mut producer = match self.0.producer().with_topic(&self.1.topic).build().await {
            Ok(p) => p,
            Err(e) => return Err(Status::internal(e.to_string())),
        };

        let message = QueuedMessage::new(request.into_inner());

        let pulsar_message = PulsarMessage {
            payload: (&message).into(),
            ..Default::default()
        };

        match producer.send(pulsar_message).await {
            Ok(sf) => {
                if let Err(e) = sf.await {
                    return Err(Status::internal(e.to_string()));
                }
            }
            Err(e) => {
                tracing::error!("enqueue error: {}", e);
                return Err(Status::internal(e.to_string()));
            }
        }

        tracing::info!("enqueue ok.");

        let response = Response::new(EnqueueResult {
            ok: true,
            result: None,
        });

        Ok(response)
    }
}
