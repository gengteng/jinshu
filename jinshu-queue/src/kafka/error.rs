use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Kafka(#[from] rdkafka::error::KafkaError),
    #[error(transparent)]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Convert(#[from] crate::error::ConvertError),
    #[error("No payload")]
    NoPayload,
}

pub type Result<T> = std::result::Result<T, Error>;
