use std::borrow::Cow;

/// 错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Join 错误
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    /// 其他错误
    #[error("{0}")]
    Other(Cow<'static, str>),
}

/// 结果
pub type Result<T> = std::result::Result<T, Error>;
