use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize)]
pub struct GatewayConfig {
    /// 监听的 IP 地址
    pub ip: IpAddr,

    /// 监听的端口号
    pub port: u16,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            ip: [0u8, 0, 0, 0].into(),
            port: 9200,
        }
    }
}

#[cfg(test)]
mod test {
    use super::GatewayConfig;

    #[test]
    fn default() {
        GatewayConfig::default();
    }
}
