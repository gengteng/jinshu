use jinshu_protocol::Pdu;
use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Login(#[from] LoginError),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Other error: {}", .0)]
    Other(Cow<'static, str>),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid token")]
    InvalidToken,
    #[error("Unexpected response: {:?}", .0)]
    UnexpectedResponse(Pdu),
    #[error("Invalid pdu: {}", .0)]
    DecodeError(#[from] jinshu_protocol::Error),
    #[error("Connection is closed")]
    ConnectionClosed,
}
