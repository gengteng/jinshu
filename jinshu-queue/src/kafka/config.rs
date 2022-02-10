use serde::{Deserialize, Serialize};

/// Kafka 生产者配置
pub type KafkaProducerConfig = KafkaConfig<ProducerConfig>;
/// Kafka 消费者配置
pub type KafkaConsumerConfig = KafkaConfig<ConsumerConfig>;

/// Kafka 配置
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

/// Kafka 生产配置
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

/// Kafka 消费配置
#[derive(Debug, Deserialize, Serialize)]
pub struct ConsumerConfig {
    /// 消费组 ID
    pub group_id: String,

    /// 分区 EOF
    pub partition_eof: bool,

    /// 偏移量自动设置
    pub auto_offset_reset: String,

    /// 会话超时时间（毫秒）
    pub session_timeout_ms: u64,

    /// 是否自动提交
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
