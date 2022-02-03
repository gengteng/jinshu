use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Pool(#[from] deadpool_redis::PoolError),
    #[error(transparent)]
    Config(#[from] deadpool_redis::ConfigError),
    #[error(transparent)]
    Build(#[from] deadpool_redis::BuildError),
    #[error(transparent)]
    Redis(#[from] redis::RedisError),
}

pub type Result<T> = std::result::Result<T, Error>;
