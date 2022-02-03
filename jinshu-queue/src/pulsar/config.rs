use serde::{Deserialize, Serialize};

pub type PulsarProducerConfig = PulsarConfig<ProducerConfig>;
pub type PulsarConsumerConfig = PulsarConfig<ConsumerConfig>;

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

/// 使用 Pulsar 作为消息队列
#[derive(Debug, Serialize, Deserialize)]
// #[serde(default)]
pub struct ProducerConfig;

impl Default for ProducerConfig {
    fn default() -> Self {
        Self
    }
}

/// 使用 Pulsar 作为消息队列
#[derive(Debug, Deserialize, Serialize)]
pub struct ConsumerConfig {
    pub consumer_name: Option<String>,

    pub consumer_id: Option<u64>,

    pub subscription_name: Option<String>,

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
