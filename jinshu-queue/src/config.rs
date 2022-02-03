use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum QueueConfig<K, P> {
    #[serde(rename = "kafka")]
    Kafka(K),

    #[serde(rename = "pulsar")]
    Pulsar(P),
}
