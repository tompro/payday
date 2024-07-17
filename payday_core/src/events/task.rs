use std::time::Duration;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinHandle;

use crate::date::{date_after, now, DateTime};
use crate::events::Result;

use super::{Message, MessageType};

/// A unique name for this task type. Not an enum so application can define their own.
pub type TaskType = String;

/// Status of a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Retrying,
    Processing,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Success,
    Failed,
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryType {
    /// Ignore the result of task execution completely. Tasks will allways be marked as successful.
    Ignore,
    /// Never retry the task but mark execution as failed if the task fails.
    Never,
    /// Retry the task with a fixed backoff.
    Fixed(u32, Duration),
    /// Retry the task with an exponential backoff.
    Exponential(u32, Duration),
}

impl RetryType {
    pub fn is_retry(&self) -> bool {
        match self {
            RetryType::Fixed(..) => true,
            RetryType::Exponential(..) => true,
            _ => false,
        }
    }
    pub fn next_retry(&self) -> Option<DateTime> {
        match self {
            RetryType::Fixed(_, d) => Some(now() + fixed_backoff(d.as_secs() as u32)),
            RetryType::Exponential(r, d) => {
                Some(now() + exponential_backoff(*r, d.as_secs() as u32))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_type: TaskType,
    pub payload: Value,
}

impl Task {
    pub fn new<T: Serialize>(task_type: TaskType, payload: T) -> Self {
        let payload = serde_json::to_value(payload).expect("could not serialize payload");
        Self { task_type, payload }
    }
}

impl Message for Task {
    fn message_type(&self) -> MessageType {
        self.task_type.to_string()
    }

    fn payload(&self) -> Value {
        self.payload.to_owned()
    }
}

/// Returns a fixed backoff duration.
pub fn fixed_backoff(offset: u32) -> Duration {
    Duration::from_secs(offset as u64)
}

/// Returns an exponential backoff duration.
pub fn exponential_backoff(count: u32, offset: u32) -> Duration {
    Duration::from_secs(offset as u64 * 2_u64.pow(count))
}
