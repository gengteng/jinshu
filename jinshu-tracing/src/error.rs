use thiserror::Error;

/// 跟踪错误
#[derive(Debug, Error)]
pub enum Error {
    /// I/O 错误
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// 设置全局跟踪器错误
    #[error(transparent)]
    Tracing(#[from] tracing::subscriber::SetGlobalDefaultError),
}

///跟踪结果
pub type Result<T> = std::result::Result<T, Error>;
