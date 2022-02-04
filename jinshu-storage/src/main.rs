use async_trait::async_trait;
use jinshu_common::Config;
use jinshu_database::config::DatabaseConfig;
use jinshu_protocol::Message;
use jinshu_queue::config::{consume_with_handler, QueueConfig};
use jinshu_queue::kafka::KafkaConsumerConfig;
use jinshu_queue::pulsar::PulsarConsumerConfig;
use jinshu_queue::{HandleResult, QueuedMessage, QueuedMessageHandler};
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use sea_orm::prelude::DateTime;
use sea_orm::ActiveModelTrait;
use sea_orm::{ActiveValue, Database, DatabaseConnection, Set};
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

#[derive(Clone)]
pub struct Storage {
    database: DatabaseConnection,
}

impl Storage {
    pub fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl QueuedMessageHandler for Storage {
    async fn handle(&self, _topic: &str, message: &QueuedMessage) -> HandleResult {
        let message = match Message::try_from(message.inner()) {
            Ok(message) => message,
            Err(e) => return HandleResult::Failure(e.to_string().into()),
        };

        let content = match serde_json::to_value(&message.content) {
            Ok(content) => content,
            Err(e) => return HandleResult::Failure(e.to_string().into()),
        };

        let model = jinshu_database::message::ActiveModel {
            id: Set(message.id.as_simple().to_string()),
            timestamp: Set(DateTime::from_timestamp(message.timestamp as i64 / 1000, 0)),
            from: Set(message.from.as_simple().to_string()),
            to: Set(message.to.as_simple().to_string()),
            content: Set(content),
            store_time: ActiveValue::NotSet,
        };

        if let Err(e) = model.insert(&self.database).await {
            return HandleResult::Failure(e.to_string().into());
        }

        HandleResult::Ok
    }
}
