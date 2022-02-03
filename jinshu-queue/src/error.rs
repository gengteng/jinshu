use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Insufficient buffer ({0} bytes)")]
    InsufficientBuffer(u64),
    #[error("Invalid content length: {0}, expected: {1}")]
    InvalidContentLength(u64, u64),
}

pub type Result<T> = std::result::Result<T, Error>;
