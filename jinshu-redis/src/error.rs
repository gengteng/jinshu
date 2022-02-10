use thiserror::Error;

/// Redis 相关错误
#[derive(Debug, Error)]
pub enum Error {
    /// 连接池错误
    #[error(transparent)]
    Pool(#[from] deadpool_redis::PoolError),
    /// 连接池配置错误
    #[error(transparent)]
    Config(#[from] deadpool_redis::ConfigError),
    /// 连接池构造错误
    #[error(transparent)]
    Build(#[from] deadpool_redis::BuildError),
    /// Redis 错误
    #[error(transparent)]
    Redis(#[from] redis::RedisError),
}

/// Redis 相关结果
pub type Result<T> = std::result::Result<T, Error>;
