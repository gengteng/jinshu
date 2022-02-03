use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Tracing(#[from] tracing::subscriber::SetGlobalDefaultError),
}

pub type Result<T> = std::result::Result<T, Error>;
