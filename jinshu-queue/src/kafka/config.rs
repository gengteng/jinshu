use serde::{Deserialize, Serialize};

pub type KafkaProducerConfig = KafkaConfig<ProducerConfig>;
pub type KafkaConsumerConfig = KafkaConfig<ConsumerConfig>;

#[derive(Debug, Deserialize, Serialize)]
pub struct KafkaConfig<T> {
    /// 服务器地址
    pub servers: String,

    /// 主题
    pub topic: String,

    /// 其他配置
    #[serde(flatten)]
    pub extension: T,
}

impl<T: Default> Default for KafkaConfig<T> {
    fn default() -> Self {
        Self {
            servers: "localhost:9092".into(),
            topic: "jinshu.dev".into(),
            extension: T::default(),
        }
    }
}

/// 使用 Kafka 作为消息队列
#[derive(Debug, Serialize, Deserialize)]
// #[serde(default)]
pub struct ProducerConfig {
    /// 超时时间
    pub message_timeout: u64,
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            message_timeout: 3000,
        }
    }
}

/// 使用 Kafka 作为消息队列
#[derive(Debug, Deserialize, Serialize)]
pub struct ConsumerConfig {
    pub group_id: String,

    pub partition_eof: bool,

    pub auto_offset_reset: String,

    pub session_timeout_ms: u64,

    pub auto_commit: bool,
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            group_id: "jinshu.group".to_string(),
            partition_eof: false,
            auto_offset_reset: "earliest".to_string(),
            session_timeout_ms: 300000,
            auto_commit: false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ConsumerConfig, KafkaConfig, ProducerConfig};

    #[test]
    fn default() {
        ConsumerConfig::default();
        ProducerConfig::default();
        KafkaConfig::<()>::default();
    }
}
