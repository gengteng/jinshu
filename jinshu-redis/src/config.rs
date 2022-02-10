use deadpool_redis::redis::{ConnectionAddr, ConnectionInfo, RedisConnectionInfo};
use deadpool_redis::PoolConfig;
use jinshu_utils::secret::Secret;
use serde::{Deserialize, Serialize};

/// Redis 的配置
#[derive(Debug, Deserialize, Serialize)]
pub struct RedisConfig {
    /// 协议
    pub schema: String,
    /// 主机名
    pub host: String,
    /// 端口
    pub port: u16,
    /// 密码
    pub password: Option<Secret>,
    /// 数据库号
    pub db_number: i64,
    /// 最大连接数
    pub max_connections: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            schema: "redis".to_string(),
            host: "localhost".to_string(),
            port: 6379,
            password: Some(Secret::new("1qaz2wsx")),
            db_number: 0,
            max_connections: 16,
        }
    }
}

impl From<RedisConfig> for deadpool_redis::Config {
    fn from(config: RedisConfig) -> Self {
        deadpool_redis::Config {
            connection: Some(
                ConnectionInfo {
                    addr: ConnectionAddr::Tcp(config.host, config.port),
                    redis: RedisConnectionInfo {
                        db: config.db_number,
                        username: None,
                        password: config.password.map(|s| s.expose().to_string()),
                    },
                }
                .into(),
            ),
            pool: Some(PoolConfig::new(config.max_connections)),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::config::RedisConfig;

    #[test]
    fn default_into() {
        let _config: deadpool_redis::Config = RedisConfig::default().into();
    }
}
