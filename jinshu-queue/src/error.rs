use thiserror::Error;

/// 消息转换错误
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// Buffer 不足
    #[error("Insufficient buffer ({0} bytes)")]
    InsufficientBuffer(u64),
    /// 内容长度不合法
    #[error("Invalid content length: {0}, expected: {1}")]
    InvalidContentLength(u64, u64),
}

/// 队列消费错误
#[derive(Debug, Error)]
pub enum Error {
    /// 消息转换错误
    #[error(transparent)]
    Convert(#[from] ConvertError),
    /// Kafka 错误
    #[error(transparent)]
    Kafka(#[from] crate::kafka::Error),
    /// Pulsar 错误
    #[error(transparent)]
    Pulsar(#[from] crate::pulsar::Error),
}

/// 队列消费结果
pub type Result<T> = std::result::Result<T, Error>;
