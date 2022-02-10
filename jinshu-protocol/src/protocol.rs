use crate::{Error, InvalidContentFormat, NoSuchCodecError};
use bytes::{Buf, BufMut, BytesMut};
use jinshu_utils::{current_millisecond, current_second};
use mime::{Mime, TEXT_PLAIN_UTF_8};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::mem::size_of;
use std::str::FromStr;
use tokio_util::codec::{Decoder, Encoder};
use url::Url;
use uuid::Uuid;

/// 事务 ID
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct TransactionId {
    time: u32,
    seq: u32,
}

/// 事务 ID 生成器
#[derive(Debug)]
pub struct TransactionIdGenerator {
    start_time: u64,
    seq: u32,
}

impl Default for TransactionIdGenerator {
    fn default() -> Self {
        Self {
            start_time: current_second(),
            seq: 0,
        }
    }
}

impl TransactionIdGenerator {
    /// 构造一个事务生成器
    pub fn new() -> Self {
        TransactionIdGenerator::default()
    }

    /// 获取当前的序列值
    pub fn seq(&self) -> u32 {
        self.seq
    }
}

impl TransactionIdGenerator {
    /// 获取下一个事务 ID
    pub fn next_id(&mut self) -> TransactionId {
        let time = (current_second() - self.start_time) as u32;
        let seq = self.seq;

        self.seq = self.seq.wrapping_add(1);

        TransactionId { time, seq }
    }
}

/// 协议数据单元
#[derive(Debug, Serialize, Deserialize)]
pub struct Pdu {
    /// 事务 ID
    pub id: TransactionId,
    /// 数据
    pub body: Body,
}

impl Pdu {
    /// 构造协议数据单元
    pub fn new(id: TransactionId, body: Body) -> Self {
        Self { id, body }
    }

    /// 是否为请求
    pub fn is_request(&self) -> bool {
        matches!(self.body, Body::Req(_))
    }

    /// 是否为响应
    pub fn is_response(&self) -> bool {
        matches!(self.body, Body::Resp(_))
    }
}

/// 协议数据单元数据
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Body {
    /// 请求
    Req(Request),
    /// 响应
    Resp(Response),
}

/// 请求
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Request {
    /// 登录
    SignIn {
        /// 锦书用户 ID
        user_id: Uuid,
        /// 登录令牌
        token: Uuid,
    },
    /// 登出
    SignOut,
    /// Ping
    Ping,
    /// 发送消息
    Send {
        /// 消息
        message: Message,
    },
    /// 推送消息
    Push {
        /// 消息
        message: Message,
    },
}

impl Request {
    /// 使用事务 ID 构造协议数据单元
    pub fn to_pdu(self, id: TransactionId) -> Pdu {
        Pdu::new(id, Body::Req(self))
    }
}

/// 响应
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum Response {
    /// 成功
    Ok,
    /// 登录成功
    SignedIn {
        /// 扩展字段
        extension: Option<serde_json::Value>,
    },
    /// 非法的令牌
    InvalidToken {
        /// 锦书用户 ID
        user_id: Uuid,
    },
    /// Pong
    Pong,
    /// 消息已入队
    Queued {
        /// 消息 ID
        id: Uuid,
    },
    /// 消息被拒绝
    Rejected {
        /// 消息 ID
        id: Uuid,
        /// 错误信息
        error: String,
    },
    /// 发生错误
    Error {
        /// 错误信息
        cause: String,
    },
}

impl Response {
    /// 使用事务 ID 构造协议数据单元
    pub fn to_pdu(self, id: TransactionId) -> Pdu {
        Pdu::new(id, Body::Resp(self))
    }
}

/// 消息
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    /// 消息 ID
    pub id: Uuid,
    /// 消息时间戳
    pub timestamp: u64,
    /// 发送者 ID
    pub from: Uuid,
    /// 接收者 ID
    pub to: Uuid,
    /// 消息内容
    pub content: Content,
}

impl Message {
    /// 构造消息
    pub fn new(from: Uuid, to: Uuid, content: Content) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: current_millisecond(),
            from,
            to,
            content,
        }
    }
}

/// 消息内容
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    /// 数据消息，包括字符串、小图片等
    Data {
        /// 数据类型
        #[serde(with = "serde_shims::mime")]
        mime: Mime,
        /// 二进制数据
        bytes: Vec<u8>,
    },
    /// 链接消息，包括大图片、视频等
    Link {
        /// 链接地址
        url: Url,
    },
}

impl Content {
    /// 使用字符串构造一个消息内容
    pub fn string(s: impl Into<String>) -> Self {
        Self::data(TEXT_PLAIN_UTF_8, s.into().into_bytes())
    }

    /// 构造一个数据消息内容
    pub fn data(mime: Mime, bytes: impl Into<Vec<u8>>) -> Self {
        Self::Data {
            mime,
            bytes: bytes.into(),
        }
    }

    /// 构造一个链接消息内容
    pub fn link(url: impl Into<Url>) -> Self {
        Self::Link { url: url.into() }
    }
}

impl TryFrom<&Content> for Vec<u8> {
    type Error = InvalidContentFormat;

    fn try_from(value: &Content) -> Result<Self, Self::Error> {
        let mut wr = Vec::with_capacity(128);
        ciborium::ser::into_writer(value, &mut wr).map_err(InvalidContentFormat::new)?;
        Ok(wr)
    }
}

impl TryFrom<&[u8]> for Content {
    type Error = InvalidContentFormat;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        ciborium::de::from_reader(value.reader()).map_err(InvalidContentFormat::new)
    }
}

/// 协议数据单元编解码器
#[derive(Debug, Copy, Clone, Default)]
pub struct PduCodec {
    codec: Codec,
    state: CodecState,
}

impl PduCodec {
    /// 构造编解码器
    pub fn new(codec: Codec) -> Self {
        Self {
            codec,
            ..Default::default()
        }
    }

    /// 获取编解码格式
    pub fn codec(&self) -> Codec {
        self.codec
    }
}

/// 编解码格式
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Codec {
    /// JSON
    #[serde(rename = "json")]
    Json = 0,
    /// MsgPack
    #[serde(rename = "msgpack")]
    MsgPack = 1,
    /// CBOR
    #[serde(rename = "cbor")]
    Cbor = 2,
    /// FlexBuffers
    #[serde(rename = "flexbuffers")]
    FlexBuffers = 3,
}

impl Default for Codec {
    fn default() -> Self {
        Self::Json
    }
}

#[derive(Debug, Clone, Copy)]
enum CodecState {
    Head,
    Data { codec: Codec, length: usize },
}

impl Default for CodecState {
    fn default() -> Self {
        Self::Head
    }
}

impl TryFrom<u8> for Codec {
    type Error = NoSuchCodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Json,
            1 => Self::MsgPack,
            2 => Self::Cbor,
            3 => Self::FlexBuffers,
            _ => return Err(NoSuchCodecError),
        })
    }
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Codec::Json => write!(f, "json"),
            Codec::MsgPack => write!(f, "msgpack"),
            Codec::Cbor => write!(f, "cbor"),
            Codec::FlexBuffers => write!(f, "flexbuffers"),
        }
    }
}

impl FromStr for Codec {
    type Err = NoSuchCodecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim() {
            "json" | "0" => Self::Json,
            "msgpack" | "1" => Self::MsgPack,
            "cbor" | "2" => Self::Cbor,
            "flexbuffers" | "3" => Self::FlexBuffers,
            _ => return Err(NoSuchCodecError),
        })
    }
}

impl Encoder<Pdu> for PduCodec {
    type Error = Error;

    #[tracing::instrument(skip(dst))]
    fn encode(&mut self, item: Pdu, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        let pdu = match self.codec {
            Codec::Json => serde_json::to_vec(&item)?,
            Codec::MsgPack => rmp_serde::to_vec(&item)?,
            Codec::Cbor => {
                let mut wr = Vec::with_capacity(128);
                ciborium::ser::into_writer(&item, &mut wr)?;
                wr
            }
            Codec::FlexBuffers => flexbuffers::to_vec(item)?,
        };

        if pdu.len() > Self::MAX_DATA_LEN {
            return Err(Error::TooLong(self.codec, pdu.len()));
        }

        tracing::debug!("serialize pdu to {} bytes {} data", pdu.len(), self.codec);

        let head = ((self.codec as u32) << 24) | (pdu.len() as u32 & 0xffffff);
        dst.put_u32(head);
        dst.put_slice(&pdu);

        Ok(())
    }
}

impl PduCodec {
    /// head is a u32
    /// | codec: u8 | length: u24 |
    pub const HEAD_LEN: usize = size_of::<u32>();

    /// Data length is a 24 bit unsigned integer
    ///
    /// MAX: 16MB
    pub const MAX_DATA_LEN: usize = (1 << 24) - 1;

    fn decode_head(&mut self, src: &mut BytesMut) -> crate::error::Result<Option<(Codec, usize)>> {
        if src.len() < Self::HEAD_LEN {
            return Ok(None);
        }

        let head = src.get_u32();
        let codec = Codec::try_from(((head & 0xff000000) >> 24) as u8)?;
        let length = (head & 0xffffff) as usize;
        src.reserve(length);
        Ok(Some((codec, length)))
    }

    fn decode_data(
        &mut self,
        src: &mut BytesMut,
        codec: Codec,
        length: usize,
    ) -> crate::error::Result<Option<Pdu>> {
        if src.len() < length {
            return Ok(None);
        }

        let bytes = src.split_to(length);
        let pdu = match codec {
            Codec::Json => serde_json::from_reader(bytes.reader())?,
            Codec::MsgPack => rmp_serde::decode::from_read(bytes.reader())?,
            Codec::Cbor => ciborium::de::from_reader(bytes.reader())?,
            Codec::FlexBuffers => flexbuffers::from_slice(bytes.as_ref())?,
        };

        tracing::debug!("deserialize {} bytes {} data to pdu", length, codec);

        Ok(Some(pdu))
    }
}

impl Decoder for PduCodec {
    type Item = Pdu;
    type Error = Error;

    #[tracing::instrument(skip(src))]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let (codec, length) = match self.state {
            CodecState::Head => match self.decode_head(src)? {
                Some((codec, length)) => {
                    self.state = CodecState::Data { codec, length };
                    (codec, length)
                }
                None => return Ok(None),
            },
            CodecState::Data { codec, length } => (codec, length),
        };

        match self.decode_data(src, codec, length)? {
            Some(pdu) => {
                self.state = CodecState::Head;
                src.reserve(Self::HEAD_LEN);
                Ok(Some(pdu))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Codec;
    use super::{Content, Message, PduCodec, Response};
    use super::{NoSuchCodecError, Pdu, Request};
    use crate::{Body, TransactionIdGenerator};
    use bytes::{BufMut, BytesMut};
    use tokio_util::codec::{Decoder, Encoder};
    use url::Url;
    use uuid::Uuid;

    #[test]
    fn codec_default() {
        assert_eq!(Codec::default(), Codec::Json);
    }

    #[test]
    fn codec_try_from_u8() {
        assert!(matches!(
            Codec::try_from(Codec::Json as u8),
            Ok(Codec::Json)
        ));
        assert!(matches!(
            Codec::try_from(Codec::Cbor as u8),
            Ok(Codec::Cbor)
        ));
        assert!(matches!(
            Codec::try_from(Codec::MsgPack as u8),
            Ok(Codec::MsgPack)
        ));
        assert!(matches!(
            Codec::try_from(Codec::FlexBuffers as u8),
            Ok(Codec::FlexBuffers)
        ));
        assert!(matches!(
            Codec::try_from(Codec::FlexBuffers as u8 + 1),
            Err(NoSuchCodecError)
        ));
    }

    #[test]
    fn codec_str() {
        assert_eq!(Codec::Json.to_string(), "json");
        assert_eq!(Codec::Cbor.to_string(), "cbor");
        assert_eq!(Codec::MsgPack.to_string(), "msgpack");
        assert_eq!(Codec::FlexBuffers.to_string(), "flexbuffers");

        assert!(matches!("json".parse(), Ok(Codec::Json)));
        assert!(matches!("cbor".parse(), Ok(Codec::Cbor)));
        assert!(matches!("msgpack".parse(), Ok(Codec::MsgPack)));
        assert!(matches!("flexbuffers".parse(), Ok(Codec::FlexBuffers)));
        assert!(matches!(
            Uuid::new_v4().to_string().parse::<Codec>(),
            Err(NoSuchCodecError)
        ));

        assert!(matches!(
            (Codec::Json as u8).to_string().parse(),
            Ok(Codec::Json)
        ));
        assert!(matches!(
            (Codec::Cbor as u8).to_string().parse(),
            Ok(Codec::Cbor)
        ));
        assert!(matches!(
            (Codec::MsgPack as u8).to_string().parse(),
            Ok(Codec::MsgPack)
        ));
        assert!(matches!(
            (Codec::FlexBuffers as u8).to_string().parse(),
            Ok(Codec::FlexBuffers)
        ));
    }

    #[test]
    fn pdu_codec_all() {
        pdu_codec(Codec::Json);
        pdu_codec(Codec::Cbor);
        pdu_codec(Codec::MsgPack);
        pdu_codec(Codec::FlexBuffers);
    }

    fn pdu_codec(codec: Codec) {
        let mut id_gen = TransactionIdGenerator::default();
        let mut codec = PduCodec::new(codec);

        let mut bytes = BytesMut::new();
        assert!(codec
            .encode(
                Request::SignIn {
                    user_id: Uuid::new_v4(),
                    token: Uuid::new_v4(),
                }
                .to_pdu(id_gen.next_id()),
                &mut bytes
            )
            .is_ok());
        assert!(matches!(
            codec.decode(&mut bytes),
            Ok(Some(Pdu {
                body: Body::Req(Request::SignIn { .. }),
                ..
            }))
        ));
        assert!(bytes.is_empty());
        assert!(matches!(codec.decode(&mut bytes), Ok(None)));

        assert!(codec
            .encode(Response::Ok.to_pdu(id_gen.next_id()), &mut bytes)
            .is_ok());

        let cur_len = bytes.len();
        assert!(codec
            .encode(
                Request::Send {
                    message: Message::new(Uuid::new_v4(), Uuid::new_v4(), Content::string("hello"))
                }
                .to_pdu(id_gen.next_id()),
                &mut bytes
            )
            .is_ok());
        let total = bytes.len();
        let mut head = bytes.split_to((total - cur_len) / 2 + cur_len);

        assert!(matches!(
            codec.decode(&mut head),
            Ok(Some(Pdu {
                body: Body::Resp(Response::Ok),
                ..
            }))
        ));
        assert!(matches!(codec.decode(&mut head), Ok(None)));
        head.put_slice(bytes.as_ref());
        assert!(matches!(
            codec.decode(&mut head),
            Ok(Some(Pdu {
                body: Body::Req(Request::Send { .. }),
                ..
            }))
        ));

        assert!(head.is_empty());

        head.put_u32(0xffffffff);
        assert!(matches!(
            codec.decode(&mut head),
            Err(crate::Error::InvalidCodec(NoSuchCodecError))
        ));
    }

    #[test]
    fn maximum() {
        let mut id_gen = TransactionIdGenerator::default();
        let mut codec = PduCodec::default();
        assert_eq!(codec.codec(), Codec::default());

        let pdu = Request::Send {
            message: Message {
                id: Uuid::new_v4(),
                timestamp: 0,
                from: Uuid::new_v4(),
                to: Uuid::new_v4(),
                content: Content::Data {
                    mime: mime::TEXT_PLAIN_UTF_8,
                    bytes: vec![b'J'; PduCodec::MAX_DATA_LEN],
                },
            },
        }
        .to_pdu(id_gen.next_id());

        let mut bytes = BytesMut::new();

        assert!(matches!(
            codec.encode(pdu, &mut bytes),
            Err(crate::Error::TooLong(Codec::Json, len)) if len > PduCodec::MAX_DATA_LEN
        ));
    }

    #[test]
    fn content() {
        let text = "hello, jinshu";

        let string = Content::string("hello, jinshu");

        let result = Vec::try_from(&string);
        assert!(result.is_ok());

        assert!(matches!(Content::try_from(result.unwrap().as_slice()),
                Ok(Content::Data { mime, bytes }) if mime == mime::TEXT_PLAIN_UTF_8 && bytes == text.as_bytes()));

        let link =
            Content::link(Url::parse("http://localhost:10000").expect("Failed to parse url"));
        assert!(matches!(link, Content::Link { .. }))
    }
}
