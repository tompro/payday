use async_trait::async_trait;

use super::{
    task::{RetryType, Task},
    Message, Result,
};

#[async_trait]
pub trait Publisher<E: Message> {
    async fn publish(&self, event: E) -> Result<()>
    where
        E: 'async_trait;
}

#[async_trait]
pub trait TaskPublisher {
    async fn once(&self, task: Task) -> Result<()>;
    async fn retry(&self, task: Task, params: RetryType) -> Result<()>;
}
