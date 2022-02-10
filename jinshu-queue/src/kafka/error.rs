use thiserror::Error;

/// Kafka 错误
#[derive(Debug, Error)]
pub enum Error {
    /// rdkafka 库报错
    #[error(transparent)]
    Kafka(#[from] rdkafka::error::KafkaError),
    /// 非法的 UTF-8 数据
    #[error(transparent)]
    InvalidUtf8(#[from] std::str::Utf8Error),
    /// 消息转换错误
    #[error(transparent)]
    Convert(#[from] crate::error::ConvertError),
    /// 没有数据的消息
    #[error("No payload")]
    NoPayload,
}

/// Kafka 结果
pub type Result<T> = std::result::Result<T, Error>;
