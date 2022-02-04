use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid SubType: {0}")]
    InvalidSubType(String),
    #[error(transparent)]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Convert(#[from] crate::error::ConvertError),
    #[error(transparent)]
    Pulsar(#[from] pulsar::Error),
    #[error(transparent)]
    PulsarConsumer(#[from] pulsar::error::ConsumerError),
}

pub type Result<T> = std::result::Result<T, Error>;
