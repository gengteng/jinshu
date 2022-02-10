use jinshu_protocol::Pdu;
use std::borrow::Cow;

/// SDK 错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O 错误
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Reqwest 库错误
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// 登录错误
    #[error(transparent)]
    Login(#[from] LoginError),
    /// 连接关闭
    #[error("Connection closed")]
    ConnectionClosed,
    /// 其他错误
    #[error("Other error: {}", .0)]
    Other(Cow<'static, str>),
}

/// SDK 结果
pub type Result<T> = std::result::Result<T, Error>;

/// 登录错误
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    /// I/O 错误
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// 非法令牌
    #[error("Invalid token")]
    InvalidToken,
    /// 异常响应
    #[error("Unexpected response: {:?}", .0)]
    UnexpectedResponse(Pdu),
    /// 解码错误
    #[error("Invalid pdu: {}", .0)]
    DecodeError(#[from] jinshu_protocol::Error),
    /// 连接关闭
    #[error("Connection is closed")]
    ConnectionClosed,
}
