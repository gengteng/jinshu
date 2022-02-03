use crate::Codec;
use std::borrow::Cow;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub struct NoSuchCodecError;

impl NoSuchCodecError {
    pub const MESSAGE: &'static str = "No such codec";
}

impl fmt::Display for NoSuchCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Self::MESSAGE)
    }
}

impl std::error::Error for NoSuchCodecError {}

#[derive(Debug)]
pub struct InvalidContentFormat(String);

impl InvalidContentFormat {
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Codec(#[from] NoSuchCodecError),
    #[error(transparent)]
    InvalidContent(#[from] InvalidContentFormat),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    MsgPackEncode(#[from] rmp_serde::encode::Error),
    #[error(transparent)]
    MsgPackDecode(#[from] rmp_serde::decode::Error),
    #[error(transparent)]
    CborDecode(#[from] ciborium::de::Error<std::io::Error>),
    #[error(transparent)]
    CborEncode(#[from] ciborium::ser::Error<std::io::Error>),
    #[error(transparent)]
    FlexBuffersEncode(#[from] flexbuffers::SerializationError),
    #[error(transparent)]
    FlexBuffersDecode(#[from] flexbuffers::DeserializationError),
    #[error("The length of {0} data ({1} bytes) exceeds the maximum length")]
    TooLong(Codec, usize),
    #[error("{0}")]
    Other(Cow<'static, str>),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use super::NoSuchCodecError;

    #[test]
    fn no_such_codec() {
        assert_eq!(NoSuchCodecError.to_string(), NoSuchCodecError::MESSAGE);
    }
}
