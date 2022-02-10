use jinshu_rpc::config::ServiceConfig;
use serde::{Deserialize, Serialize};

/// Receiver 的配置
#[derive(Debug, Deserialize, Serialize)]
// #[serde(default)]
pub struct ReceiverConfig {
    /// 接收服务配置
    #[serde(flatten)]
    pub service: ServiceConfig,
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self {
            service: ServiceConfig {
                service_name: "receiver".into(),
                public_host: "0.0.0.0".into(),
                listen_ip: [0u8, 0, 0, 0].into(),
                listen_port: 9100,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::ReceiverConfig;

    #[test]
    fn default() {
        ReceiverConfig::default();
    }
}
