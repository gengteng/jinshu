use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error("{0}")]
    Other(Cow<'static, str>),
}

pub type Result<T> = std::result::Result<T, Error>;
