use std::borrow::Cow;

/// jinshu-common 的错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O 错误
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// 配置错误
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    /// 其他错误
    #[error("{0}")]
    Other(Cow<'static, str>),
}

/// jinshu-common 的结果
pub type Result<T> = std::result::Result<T, Error>;
