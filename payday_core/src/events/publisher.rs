use async_trait::async_trait;

use super::{Event, Result};

#[async_trait]
pub trait Publisher<E: Event> {
    async fn publish(&self, event: E) -> Result<()>;
}

#[async_trait]
pub trait Handler<E: Event> {
    async fn handle(&self, event: E) -> Result<()>;
}
