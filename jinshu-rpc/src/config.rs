use http::Uri;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpListener;

/// 服务配置
#[derive(Debug, Deserialize, Serialize)]
// #[serde(default)]
pub struct ServiceConfig {
    /// 服务名
    pub service_name: String,

    /// 外部调用时使用的 host
    ///
    /// 如果设置为 `0.0.0.0`，则会获取本地网卡的 IP 地址作为公开访问的 host
    ///
    pub public_host: String,

    /// 监听的 IP 地址
    pub listen_ip: IpAddr,

    /// 接收服务监听的端口
    pub listen_port: u16,
}

impl ServiceConfig {
    /// 尝试绑定端口
    pub async fn try_bind(&self) -> anyhow::Result<(TcpListener, Uri)> {
        let address = SocketAddr::new(self.listen_ip, self.listen_port);

        let listener = TcpListener::bind(address).await?;
        let port = listener.local_addr()?.port();

        let service_uri = match self.public_host.parse::<IpAddr>() {
            Ok(public_ip) if public_ip.is_unspecified() => {
                match jinshu_utils::get_all_ip_addr()?.as_slice() {
                    [ip, ..] => {
                        tracing::info!(%ip,
                            "The public host is an unspecified address, use local interface ip address."
                        );
                        format!("http://{}:{}/", ip, port).parse()?
                    }
                    _ => {
                        anyhow::bail!("Failed to get local interface ip address, please specify 'public_host' in the configuration file")
                    }
                }
            }
            _ => format!("http://{}:{}/", self.public_host, port).parse()?,
        };

        Ok((listener, service_uri))
    }
}
