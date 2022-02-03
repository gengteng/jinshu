use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("{0}")]
    Other(Cow<'static, str>),
}

pub type Result<T> = std::result::Result<T, Error>;
