mod config;
mod consumer;
mod error;

use crate::QueuedMessage;
pub use config::*;
pub use consumer::*;
pub use error::*;
use rdkafka::message::BorrowedMessage;
use rdkafka::Message as _;

impl<'a> TryFrom<&BorrowedMessage<'a>> for QueuedMessage {
    type Error = error::Error;

    fn try_from(value: &BorrowedMessage<'a>) -> error::Result<Self> {
        match value.payload() {
            Some(bytes) => Ok(QueuedMessage::try_from(bytes)?),
            None => Err(error::Error::NoPayload),
        }
    }
}
