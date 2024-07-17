use std::fmt::Debug;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

pub mod handler;
pub mod publisher;
pub mod task;

pub type Result<T> = std::result::Result<T, MessageError>;

#[derive(Debug)]
pub enum MessageError {
    PublishError(String),
    SubscribeError(String),
    ConfirmError(String),
}

/// A unique name for this event type. Not an enum so application can define their own.
pub type MessageType = String;

/// A message is a type that can be published to a stream or queue.
pub trait Message: DeserializeOwned + Serialize + Clone + Send + Sync + Debug {
    fn message_type(&self) -> MessageType;
    fn payload(&self) -> Value;
}
