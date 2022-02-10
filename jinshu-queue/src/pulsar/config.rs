use serde::{Deserialize, Serialize};

/// Pulsar 生产者配置
pub type PulsarProducerConfig = PulsarConfig<ProducerConfig>;
/// Pulsar 消费者配置
pub type PulsarConsumerConfig = PulsarConfig<ConsumerConfig>;

/// Pulsar 配置
#[derive(Debug, Serialize, Deserialize)]
pub struct PulsarConfig<T> {
    /// 服务器地址
    pub url: url::Url,

    /// 主题
    pub topic: String,

    /// 其他配置
    #[serde(flatten)]
    pub extension: T,
}

impl<T: Default> Default for PulsarConfig<T> {
    fn default() -> Self {
        Self {
            url: url::Url::parse("pulsar://localhost:6650")
                .expect("Failed to parse default pulsar url."),
            topic: "persistent://public/default/jinshu.dev".into(),
            extension: T::default(),
        }
    }
}

/// Pulsar 生产配置
#[derive(Debug, Serialize, Deserialize)]
// #[serde(default)]
pub struct ProducerConfig;

impl Default for ProducerConfig {
    fn default() -> Self {
        Self
    }
}

/// Pulsar 消费配置
#[derive(Debug, Deserialize, Serialize)]
pub struct ConsumerConfig {
    /// 消费者名
    pub consumer_name: Option<String>,

    /// 消费者 ID
    pub consumer_id: Option<u64>,

    /// 订阅名
    pub subscription_name: Option<String>,

    /// 订阅类型
    pub subscription_type: String,
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            consumer_name: None,
            consumer_id: None,
            subscription_name: None,
            subscription_type: "keyshared".into(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ConsumerConfig, ProducerConfig, PulsarConfig};

    #[test]
    fn default() {
        ConsumerConfig::default();
        ProducerConfig::default();
        PulsarConfig::<()>::default();
    }
}
