mod config;
mod consumer;
mod error;

pub use config::*;
pub use consumer::*;
pub use error::*;
use pulsar::{DeserializeMessage, Payload};

impl DeserializeMessage for crate::QueuedMessage {
    type Output = error::Result<Self>;

    fn deserialize_message(payload: &Payload) -> Self::Output {
        Ok(crate::QueuedMessage::try_from(payload.data.as_slice())?)
    }
}
