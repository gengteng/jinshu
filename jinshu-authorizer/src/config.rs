use jinshu_rpc::config::ServiceConfig;
use serde::{Deserialize, Serialize};

/// Authorizer 的配置
#[derive(Debug, Deserialize, Serialize)]
// #[serde(default)]
pub struct AuthorizerConfig {
    /// Authorizer 服务配置
    #[serde(flatten)]
    pub service: ServiceConfig,
}

impl Default for AuthorizerConfig {
    fn default() -> Self {
        Self {
            service: ServiceConfig {
                service_name: "authorizer".into(),
                public_host: "0.0.0.0".into(),
                listen_ip: [0u8, 0, 0, 0].into(),
                listen_port: 9300,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::AuthorizerConfig;

    #[test]
    fn default() {
        AuthorizerConfig::default();
    }
}
