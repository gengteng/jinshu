use async_trait::async_trait;
use jinshu_protocol::Message;
use jinshu_queue::{HandleResult, QueuedMessage, QueuedMessageHandler};
use sea_orm::prelude::TimeDateTimeWithTimeZone;
use sea_orm::ActiveModelTrait;
use sea_orm::{ActiveValue, DatabaseConnection, Set};
use std::time::Duration;

/// 消息存储器
#[derive(Clone)]
pub struct Storage<B> {
    backend: B,
}

impl<B> Storage<B> {
    /// 构造数据库消息存储器
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl QueuedMessageHandler for Storage<DatabaseConnection> {
    async fn handle(&self, _topic: &str, message: &QueuedMessage) -> HandleResult {
        let message = match Message::try_from(message.inner()) {
            Ok(message) => message,
            Err(e) => return HandleResult::Failure(e.to_string().into()),
        };

        let content = match serde_json::to_value(&message.content) {
            Ok(content) => content,
            Err(e) => return HandleResult::Failure(e.to_string().into()),
        };

        let secs = message.timestamp as i64 / 1000;
        let nsecs = (message.timestamp as i64 - (secs * 1000)) as u64 * 1_000_000;
        let model = jinshu_database::message::ActiveModel {
            id: Set(message.id.as_simple().to_string()),
            timestamp: Set(
                TimeDateTimeWithTimeZone::from_unix_timestamp(secs) + Duration::from_nanos(nsecs)
            ),
            from: Set(message.from.as_simple().to_string()),
            to: Set(message.to.as_simple().to_string()),
            content: Set(content),
            store_time: ActiveValue::NotSet,
        };

        if let Err(e) = model.insert(&self.backend).await {
            return HandleResult::Failure(e.to_string().into());
        }

        HandleResult::Ok
    }
}
