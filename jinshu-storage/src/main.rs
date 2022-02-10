use jinshu_common::Config;
use jinshu_database::config::DatabaseConfig;
use jinshu_queue::config::{consume_with_handler, QueueConfig};
use jinshu_queue::kafka::KafkaConsumerConfig;
use jinshu_queue::pulsar::PulsarConsumerConfig;
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use sea_orm::Database;

use jinshu_storage::Storage;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Conf {
    tracing: TracingConfig,
    database: DatabaseConfig,

    /// 使用的消息队列中间件
    #[serde(flatten)]
    queue: QueueConfig<KafkaConsumerConfig, PulsarConsumerConfig>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracing = conf.tracing.init("storage")?;

    tracing::info!(?conf);

    let Conf {
        database, queue, ..
    } = conf;

    tracing::info!(?database);
    let database = Database::connect(database).await?;

    let storage = Storage::new(database);

    consume_with_handler(queue, storage, shutdown_signal()).await?;

    Ok(())
}
