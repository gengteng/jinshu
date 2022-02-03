use crate::domain::message::Message as RpcMessage;
use jinshu_protocol::{Content, Message};
use uuid::Uuid;

impl TryFrom<&Message> for RpcMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Message) -> Result<Self, Self::Error> {
        Ok(Self {
            id: message.id.as_bytes().to_vec(),
            timestamp: message.timestamp,
            from: message.from.as_bytes().to_vec(),
            to: message.to.as_bytes().to_vec(),
            content: Vec::<u8>::try_from(&message.content)?,
        })
    }
}

impl TryFrom<&RpcMessage> for Message {
    type Error = anyhow::Error;

    fn try_from(msg: &RpcMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::from_slice(&msg.id)?,
            timestamp: msg.timestamp,
            from: Uuid::from_slice(&msg.from)?,
            to: Uuid::from_slice(&msg.to)?,
            content: Content::try_from(msg.content.as_slice())?,
        })
    }
}
