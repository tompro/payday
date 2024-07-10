use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

pub mod publisher;

pub type Result<T> = std::result::Result<T, EventError>;

#[derive(Debug)]
pub enum EventError {
    PublishError(String),
    SubscribeError(String),
}

/// A unique name for this event type. Not an enum so application can define their own.
pub type EventType = String;

pub trait Event: DeserializeOwned + Serialize + Clone + Send + Sync {
    fn event_type(&self) -> EventType;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericEvent {
    event_type: EventType,
    pub payload: Value,
}

impl GenericEvent {
    pub fn new(event_type: EventType, payload: Value) -> Self {
        Self {
            event_type,
            payload,
        }
    }
}

impl Event for GenericEvent {
    fn event_type(&self) -> EventType {
        self.event_type.to_owned()
    }
}
