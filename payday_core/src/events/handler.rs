use crate::events::{
    task::{Task, TaskResult},
    Message, Result,
};
use async_trait::async_trait;
use tokio::task::JoinHandle;

#[async_trait]
pub trait Handler<E: Message> {
    async fn handle(&self, event: E) -> Result<()>
    where
        E: 'async_trait;
}

#[async_trait]
pub trait TaskHandler: Send + Sync {
    fn allow_retry(&self) -> bool;
    fn allow_recovery(&self) -> bool;
    fn handles(&self, task_type: &str) -> bool;
    async fn handle(&self, task: Task) -> Result<TaskResult>;
}

pub struct PrintTaskHandler;

#[async_trait]
impl TaskHandler for PrintTaskHandler {
    fn allow_retry(&self) -> bool {
        true
    }

    fn allow_recovery(&self) -> bool {
        true
    }

    fn handles(&self, _task_type: &str) -> bool {
        true
    }

    async fn handle(&self, task: Task) -> Result<TaskResult> {
        println!("Task: {:?}", task);
        Ok(TaskResult::Success)
    }
}

#[async_trait]
pub trait MessageProcessorApi: Send + Sync {
    async fn process(&self) -> Result<JoinHandle<Result<()>>>;
}
