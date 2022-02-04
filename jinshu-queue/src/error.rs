use thiserror::Error;

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("Insufficient buffer ({0} bytes)")]
    InsufficientBuffer(u64),
    #[error("Invalid content length: {0}, expected: {1}")]
    InvalidContentLength(u64, u64),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Convert(#[from] ConvertError),
    #[error(transparent)]
    Kafka(#[from] crate::kafka::Error),
    #[error(transparent)]
    Pulsar(#[from] crate::pulsar::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
