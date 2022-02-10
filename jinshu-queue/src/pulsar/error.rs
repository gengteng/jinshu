use thiserror::Error;

/// Pulsar 错误
#[derive(Debug, Error)]
pub enum Error {
    /// 不合法的订阅类型
    #[error("Invalid SubType: {0}")]
    InvalidSubType(String),
    /// 非法的 UTF-8 数据
    #[error(transparent)]
    InvalidUtf8(#[from] std::str::Utf8Error),
    /// 数据转换错误
    #[error(transparent)]
    Convert(#[from] crate::error::ConvertError),
    /// pulsar 库报错
    #[error(transparent)]
    Pulsar(#[from] pulsar::Error),
    /// pulsar 消费者错误
    #[error(transparent)]
    PulsarConsumer(#[from] pulsar::error::ConsumerError),
}

/// Pulsar 结果
pub type Result<T> = std::result::Result<T, Error>;
