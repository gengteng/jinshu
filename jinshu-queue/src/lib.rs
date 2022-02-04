use jinshu_rpc::domain::message::Message as RpcMessage;
use std::borrow::Cow;
use std::fmt::Debug;
use std::mem::size_of;
use uuid::Uuid;

pub mod config;
pub mod error;
pub mod kafka;
pub mod pulsar;

/// 对消费到的消息进行处理得到的结果
///
#[derive(Debug)]
pub enum HandleResult {
    /// 处理成功
    Ok,

    /// 处理失败，但是可以继续消费
    Failure(Cow<'static, str>),

    /// 处理发生错误，停止消费
    Error(Cow<'static, str>),
}

#[derive(Debug, Clone)]
pub struct QueuedMessage(RpcMessage);

impl QueuedMessage {
    pub fn new(message: RpcMessage) -> Self {
        Self(message)
    }

    pub fn unbox(self) -> RpcMessage {
        self.0
    }

    pub fn inner(&self) -> &RpcMessage {
        &self.0
    }
}

const CONTENT_LEN_OFFSET: u64 = size_of::<Uuid>() as u64 * 3 + 8; // id + ts + from + to
const CONTENT_LEN_END: u64 = CONTENT_LEN_OFFSET + size_of::<u64>() as u64; // CONTENT_LEN_OFFSET + size of content len (u64)

impl TryFrom<&[u8]> for QueuedMessage {
    type Error = crate::error::ConvertError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let len = value.len() as u64;
        if len < CONTENT_LEN_END {
            return Err(Self::Error::InsufficientBuffer(len));
        }

        let content_len = (&value[CONTENT_LEN_OFFSET as usize..CONTENT_LEN_END as usize])
            .try_into()
            .unwrap();
        let content_len = u64::from_be_bytes(content_len);

        let expected_content_len = len - CONTENT_LEN_END;
        if expected_content_len != content_len {
            return Err(Self::Error::InvalidContentLength(
                content_len,
                expected_content_len,
            ));
        }

        let mut pos = 0;
        let id = Vec::from(&value[pos..pos + size_of::<Uuid>()]);
        pos += size_of::<Uuid>();

        let timestamp = (&value[pos..pos + size_of::<u64>()]).try_into().unwrap();
        let timestamp = u64::from_be_bytes(timestamp);

        pos += size_of::<u64>();
        let from = Vec::from(&value[pos..pos + size_of::<Uuid>()]);

        pos += size_of::<Uuid>();
        let to = Vec::from(&value[pos..pos + size_of::<Uuid>()]);

        pos += size_of::<Uuid>() + size_of::<u64>();
        let content = Vec::from(&value[pos..]);

        Ok(Self(RpcMessage {
            id,
            timestamp,
            from,
            to,
            content,
        }))
    }
}

impl From<&QueuedMessage> for Vec<u8> {
    fn from(QueuedMessage(msg): &QueuedMessage) -> Self {
        let mut vec = Vec::with_capacity(128);
        vec.extend_from_slice(&msg.id);
        vec.extend_from_slice(&msg.timestamp.to_be_bytes()); // big-endian
        vec.extend_from_slice(&msg.from);
        vec.extend_from_slice(&msg.to);
        vec.extend_from_slice(&(msg.content.len() as u64).to_be_bytes());
        vec.extend_from_slice(&msg.content);
        vec
    }
}

#[async_trait::async_trait]
pub trait QueuedMessageHandler {
    async fn handle(&self, topic: &str, message: &QueuedMessage) -> HandleResult;
}

#[cfg(test)]
mod test {
    use crate::QueuedMessage;
    use jinshu_protocol::Content;
    use jinshu_rpc::domain::message::Message as RpcMessage;
    use jinshu_utils::current_millisecond;
    use url::Url;
    use uuid::Uuid;

    #[test]
    fn message_vec() {
        let content = Content::Link {
            url: Url::parse("http://localhost:8765/index.html").unwrap(),
        };

        let convert = Vec::try_from(&content);
        assert!(convert.is_ok());

        let message = RpcMessage {
            id: Uuid::new_v4().as_bytes().to_vec(),
            timestamp: current_millisecond(),
            from: Uuid::new_v4().as_bytes().to_vec(),
            to: Uuid::new_v4().as_bytes().to_vec(),
            content: convert.unwrap(),
        };

        let qm = QueuedMessage::new(message);

        let vec = Vec::from(&qm);
        assert!(matches!(
            QueuedMessage::try_from(vec.as_slice()),
            Ok(m) if m.0.id == qm.0.id && m.0.timestamp == qm.0.timestamp
                && m.0.from == qm.0.from && m.0.to == qm.0.to
                && m.0.content == qm.0.content
        ));
    }
}
