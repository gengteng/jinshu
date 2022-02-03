use jinshu_protocol::Codec;
use jinshu_rpc::config::ServiceConfig;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize)]
//#[serde(default)]
pub struct CometConfig {
    /// 监听的 IP 地址
    pub ip: IpAddr,

    /// 监听的端口号
    pub port: u16,

    /// 使用的编码, 0.json | 1.msgpack | 2.cbor | 3. flexbuffers
    pub codec: Codec,

    /// 服务配置
    pub service: ServiceConfig,

    /// 要消费的 Receiver 服务名
    pub receiver_name: String,

    /// 要消费的 Authorizer 服务名
    pub authorizer_name: String,
}

impl Default for CometConfig {
    fn default() -> Self {
        Self {
            ip: [0u8, 0, 0, 0].into(),
            port: 9000,
            codec: Codec::Json,
            service: ServiceConfig {
                service_name: "comet".into(),
                public_host: "0.0.0.0".into(),
                listen_ip: [0u8, 0, 0, 0].into(),
                listen_port: 9400,
            },
            receiver_name: "receiver".into(),
            authorizer_name: "authorizer".into(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::CometConfig;

    #[test]
    fn default() {
        CometConfig::default();
    }
}
