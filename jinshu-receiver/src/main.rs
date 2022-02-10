use jinshu_common::Config;
use jinshu_queue::config::QueueConfig;
use jinshu_queue::kafka::KafkaProducerConfig;
use jinshu_queue::pulsar::PulsarProducerConfig;
use jinshu_receiver::KafkaReceiver;
use jinshu_receiver::PulsarReceiver;
use jinshu_receiver::ReceiverConfig;
use jinshu_rpc::receiver::receiver_server::ReceiverServer;
use jinshu_rpc::registry::etcd::{EtcdConfig, EtcdRegistry};
use jinshu_rpc::registry::Registry;
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Conf {
    receiver: ReceiverConfig,
    tracing: TracingConfig,
    etcd: EtcdConfig,
    /// 使用的消息队列中间件
    #[serde(flatten)]
    queue: QueueConfig<KafkaProducerConfig, PulsarProducerConfig>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracer = conf.tracing.init("receiver")?;

    tracing::info!(?conf);

    let Conf {
        receiver: ReceiverConfig { service },
        etcd,
        queue,
        ..
    } = conf;

    let registry = EtcdRegistry::new(&etcd).await?;

    let (uri, handle) = match queue {
        QueueConfig::Kafka(config) => {
            let kp = KafkaReceiver::create(config)?;
            registry
                .run_service(service, ReceiverServer::new(kp), shutdown_signal())
                .await?
        }
        QueueConfig::Pulsar(config) => {
            let pp = PulsarReceiver::create(config).await?;
            registry
                .run_service(service, ReceiverServer::new(pp), shutdown_signal())
                .await?
        }
    };

    tracing::info!(%uri, "Receiver service is running.");
    handle.await?;

    Ok(())
}
