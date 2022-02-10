use crate::Codec;
use std::borrow::Cow;
use std::fmt;
use std::fmt::Display;

/// 没有这种编码格式的错误
#[derive(Debug)]
pub struct NoSuchCodecError;

impl NoSuchCodecError {
    /// 没有这种编码格式的错误信息
    pub const MESSAGE: &'static str = "No such codec";
}

impl fmt::Display for NoSuchCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Self::MESSAGE)
    }
}

impl std::error::Error for NoSuchCodecError {}

/// 不合法的消息内容格式错误
#[derive(Debug)]
pub struct InvalidContentFormat(String);

impl InvalidContentFormat {
    /// 构造一个不合法的消息内容格式错误
    pub fn new<D: Display>(msg: D) -> Self {
        Self(msg.to_string())
    }
}

impl fmt::Display for InvalidContentFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for InvalidContentFormat {}

/// 协议相关错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O 错误
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// 不存在编码错误
    #[error(transparent)]
    InvalidCodec(#[from] NoSuchCodecError),
    /// 格式不合法错误
    #[error(transparent)]
    InvalidContent(#[from] InvalidContentFormat),
    /// Json 错误
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// MsgPack 编码错误
    #[error(transparent)]
    MsgPackEncode(#[from] rmp_serde::encode::Error),
    /// MsgPack 解码错误
    #[error(transparent)]
    MsgPackDecode(#[from] rmp_serde::decode::Error),
    /// CBOR 解码错误
    #[error(transparent)]
    CborDecode(#[from] ciborium::de::Error<std::io::Error>),
    /// CBOR 编码错误
    #[error(transparent)]
    CborEncode(#[from] ciborium::ser::Error<std::io::Error>),
    /// FlexBuffers 编码错误
    #[error(transparent)]
    FlexBuffersEncode(#[from] flexbuffers::SerializationError),
    /// FlexBuffers 解码错误
    #[error(transparent)]
    FlexBuffersDecode(#[from] flexbuffers::DeserializationError),
    /// 报文过长错误
    #[error("The length of {0} data ({1} bytes) exceeds the maximum length")]
    TooLong(Codec, usize),
    /// 其他错误
    #[error("{0}")]
    Other(Cow<'static, str>),
}

/// 协议结果
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use super::NoSuchCodecError;

    #[test]
    fn no_such_codec() {
        assert_eq!(NoSuchCodecError.to_string(), NoSuchCodecError::MESSAGE);
    }
}
