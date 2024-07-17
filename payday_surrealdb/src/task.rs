use crate::serialize_chrono_as_sql_datetime;
use crate::serialize_chrono_as_sql_datetime_option;
use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use payday_core::events::task::TaskResult;
use payday_core::{
    date::{now, DateTime},
    events::{
        handler::{Handler, MessageProcessorApi, TaskHandler},
        publisher::{Publisher, TaskPublisher},
        task::{RetryType, Task, TaskStatus, TaskType},
        Message, MessageError, MessageType, Result,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::{engine::any::Any, sql::Thing, Surreal};
use tokio::{sync::Mutex, task::JoinHandle};

/// A task stored in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealTask {
    pub id: Option<Thing>,
    pub message_type: TaskType,
    pub task_type: TaskType,
    pub payload: Task,
    pub status: TaskStatus,
    pub processed: bool,
    pub retry_type: RetryType,
    pub max_retry: Option<u32>,
    pub num_retry: u32,
    #[serde(serialize_with = "serialize_chrono_as_sql_datetime_option")]
    pub next_retry: Option<DateTime>,
    #[serde(serialize_with = "serialize_chrono_as_sql_datetime")]
    pub received_at: DateTime,
    #[serde(serialize_with = "serialize_chrono_as_sql_datetime_option")]
    pub started_at: Option<DateTime>,
    #[serde(serialize_with = "serialize_chrono_as_sql_datetime_option")]
    pub completed_at: Option<DateTime>,
    pub recover_after: Option<i64>,
}

impl SurrealTask {
    pub fn new(payload: Task, retry_type: RetryType) -> Self {
        let max_retry = match retry_type {
            RetryType::Fixed(r, ..) => Some(r),
            RetryType::Exponential(r, ..) => Some(r),
            _ => None,
        };

        Self {
            id: None,
            message_type: "task".to_string(),
            task_type: payload.task_type.to_owned(),
            retry_type,
            max_retry,
            payload,
            status: TaskStatus::Pending,
            processed: false,
            num_retry: 0,
            next_retry: None,
            received_at: now(),
            started_at: None,
            completed_at: None,
            recover_after: None,
        }
    }

    pub fn task_type(&self) -> TaskType {
        self.payload.task_type.to_owned()
    }

    pub fn should_retry(&self) -> bool {
        self.max_retry.is_some()
            && self.max_retry.unwrap() > 0
            && self.num_retry < self.max_retry.unwrap()
            && (self.retry_type.is_retry())
    }

    pub fn update_status(&self, result: TaskResult) -> SurrealTask {
        let mut updated = self.clone();
        match result {
            TaskResult::Success => {
                updated.status = TaskStatus::Succeeded;
                updated.completed_at = Some(now());
                updated.processed = true;
            }
            TaskResult::Retry => {
                if updated.should_retry() {
                    updated.status = TaskStatus::Retrying;
                    updated.next_retry = updated.retry_type.next_retry();
                    updated.num_retry += 1;
                } else {
                    updated.status = TaskStatus::Failed;
                    updated.completed_at = Some(now());
                    updated.processed = true;
                }
            }
            TaskResult::Failed => {
                updated.status = TaskStatus::Failed;
                updated.completed_at = Some(now());
                updated.processed = true;
            }
        };
        updated
    }
}

impl Message for SurrealTask {
    fn message_type(&self) -> MessageType {
        self.task_type.to_string()
    }

    fn payload(&self) -> Value {
        serde_json::to_value(self.payload.to_owned()).expect("could not serialize task")
    }
}

pub struct SurrealTaskQueue {
    db: Surreal<Any>,
    task_table: String,
}

impl SurrealTaskQueue {
    pub fn new(db: Surreal<Any>, task_table: &str) -> Self {
        Self {
            db,
            task_table: task_table.to_string(),
        }
    }

    async fn publish_task(&self, task: SurrealTask) -> Result<()> {
        let res: Vec<SurrealTask> = self
            .db
            .create(&self.task_table)
            .content(task)
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

#[async_trait]
impl<E: Message> Publisher<E> for SurrealTaskQueue {
    async fn publish(&self, event: E) -> Result<()>
    where
        E: 'async_trait,
    {
        let task = SurrealTask::new(
            Task::new(event.message_type(), event.payload()),
            RetryType::Ignore,
        );
        self.publish_task(task).await
    }
}

#[async_trait]
impl TaskPublisher for SurrealTaskQueue {
    async fn once(&self, task: Task) -> Result<()> {
        let task = SurrealTask::new(task, RetryType::Never);
        self.publish_task(task).await?;
        Ok(())
    }

    async fn retry(&self, task: Task, params: RetryType) -> Result<()> {
        let task = SurrealTask::new(task, params);
        self.publish_task(task).await?;
        Ok(())
    }
}

pub struct SurrealTaskProcessor {
    db: Surreal<Any>,
    task_table: String,
    handlers: Vec<Arc<Mutex<dyn TaskHandler>>>,
    poll_interval: Duration,
    batch_size: usize,
    task_types: Option<Vec<String>>,
}

impl SurrealTaskProcessor {
    pub fn new(
        db: Surreal<Any>,
        task_table: &str,
        handlers: Vec<Arc<Mutex<dyn TaskHandler>>>,
    ) -> Self {
        Self {
            db,
            task_table: task_table.to_string(),
            handlers,
            poll_interval: Duration::from_secs(1),
            batch_size: 5,
            task_types: None,
        }
    }

    pub fn add_handler(&mut self, handler: Arc<Mutex<dyn TaskHandler>>) {
        self.handlers.push(handler);
    }

    async fn query_batch(&self) -> Result<Vec<SurrealTask>> {
        let mut response = self
            .db
            .query(task_query(
                &self.task_table,
                self.batch_size,
                self.task_types.clone(),
            ))
            .await
            .map_err(|e| MessageError::SubscribeError(e.to_string()))?;
        Ok(response
            .take(0)
            .map_err(|e| MessageError::SubscribeError(e.to_string()))?)
    }
}

#[async_trait]
impl MessageProcessorApi for SurrealTaskProcessor {
    async fn process(&self) -> Result<JoinHandle<Result<()>>> {
        let db = self.db.clone();
        let table = self.task_table.to_string();
        let batch_size = self.batch_size;
        let task_types = self.task_types.clone();
        let handlers = self.handlers.clone();
        let interval = self.poll_interval.clone();

        let handle = tokio::spawn(async move {
            loop {
                let tasks = query_batch(db.clone(), &table, batch_size, task_types.clone()).await?;
                for task in tasks {
                    for handler in handlers.iter() {
                        let h = handler.lock().await;
                        if h.handles(&task.task_type) {
                            let updated = match h.handle(task.payload.clone()).await {
                                Ok(res) => task.update_status(res),
                                Err(_) if task.should_retry() => {
                                    task.update_status(TaskResult::Retry)
                                }
                                _ => task.update_status(TaskResult::Failed),
                            };

                            let _: Option<SurrealTask> = db
                                .clone()
                                .update((&table, task.id.clone().unwrap().id))
                                .content(updated)
                                .await
                                .map_err(|e| MessageError::ConfirmError(e.to_string()))?;
                        }
                    }
                }
                tokio::time::sleep(interval).await;
            }
            Ok(())
        });
        Ok(handle)
    }
}

async fn query_batch(
    db: Surreal<Any>,
    table: &str,
    limit: usize,
    task_types: Option<Vec<String>>,
) -> Result<Vec<SurrealTask>> {
    let query = task_query(table, limit, task_types.clone());
    let mut response = db
        .query(query)
        .await
        .map_err(|e| MessageError::SubscribeError(e.to_string()))?;
    Ok(response
        .take(0)
        .map_err(|e| MessageError::SubscribeError(e.to_string()))?)
}

async fn cleanup_batch(
    db: Surreal<Any>,
    table: &str,
    task_types: Option<Vec<String>>,
) -> Result<()> {
    let response = db
        .query(task_cleanup_query(table, task_types.clone()))
        .await
        .map_err(|e| MessageError::SubscribeError(e.to_string()))?;
    Ok(())
}

fn task_query(table: &str, limit: usize, task_types: Option<Vec<String>>) -> String {
    format!(
        "BEGIN TRANSACTION; \
        let $batch = SELECT * FROM {} \
            WHERE processed = false \
            AND status INSIDE ['Pending', 'Retrying'] \
            AND (next_retry = NONE OR next_retry < time::now()) \
            {} \
            LIMIT {}; \
         UPDATE $batch SET status = 'Processing', started_at = time::now(); \
         RETURN $batch; \
         COMMIT TRANSACTION; \
    ",
        table,
        task_type_query_fragment(task_types),
        limit
    )
}

fn task_cleanup_query(table: &str, task_types: Option<Vec<String>>) -> String {
    format!(
        "BEGIN TRANSACTION; \
         let $failed = SELECT * FROM {}
         WHERE processed = false 
         AND status = 'Processing'
         AND started_at + (retry_after OR $max_execution) < time::now()
         {};
         UPDATE $failed SET status = 'Failed', processed = true; \
         COMMIT TRANSACTION; \
    ",
        table,
        task_type_query_fragment(task_types)
    )
}

fn task_type_query_fragment(task_types: Option<Vec<String>>) -> String {
    match task_types {
        Some(types) if !types.is_empty() => {
            let task_type_string = format!("\"{}\"", types.join("\",\""));
            format!("AND task_type INSIDE [{}]", task_type_string)
        }
        _ => "".to_string(),
    }
}
