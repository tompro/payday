use async_trait::async_trait;
use payday_core::{
    events::{publisher::Publisher, task::Task, Message, MessageError, Result},
    PaydayResult,
};
use surrealdb::{engine::any::Any, Notification, Surreal};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

use crate::task::SurrealTask;

pub struct EventStream {
    db: Surreal<Any>,
    event_table: String,
}

impl EventStream {
    pub fn new(db: Surreal<Any>, event_table: &str) -> Self {
        Self {
            db,
            event_table: event_table.to_string(),
        }
    }

    pub async fn subscribe(&self) -> PaydayResult<JoinHandle<()>> {
        let table = self.event_table.to_string();
        let db = self.db.clone();
        let handle = tokio::spawn(async move {
            let mut stream = db.select(&table).live().await.unwrap();
            while let Some(event) = stream.next().await {
                print_event(event);
            }
        });
        Ok(handle)
    }
}

fn print_event(event: surrealdb::Result<Notification<SurrealTask>>) {
    println!("Event: {:?}", event);
}

#[async_trait]
impl<E: Message> Publisher<E> for EventStream {
    async fn publish(&self, event: E) -> Result<()>
    where
        E: 'async_trait,
    {
        let res: Vec<E> = self
            .db
            .create(&self.event_table)
            .content(event)
            .await
            .map_err(|e| MessageError::PublishError(e.to_string()))?;
        if res.is_empty() {
            Err(MessageError::PublishError(
                "event was not inserted".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}
