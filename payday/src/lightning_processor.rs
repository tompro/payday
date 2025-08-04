use std::sync::Arc;

use payday_core::api::lightning_api::{
    LightningTransactionEventHandler, LightningTransactionEventProcessorApi,
    LightningTransactionStreamApi,
};
use payday_core::persistence::offset::OffsetStoreApi;
use payday_core::processor::lightning_processor::LightningTransactionProcessor;
use payday_postgres::offset::OffsetStore;
use sqlx::{Pool, Postgres};
use tokio::task::{JoinError, JoinSet};
use tracing::{error, info};

#[allow(dead_code)]
pub struct LightningEventProcessor {
    pool: Pool<Postgres>,
    nodes: Vec<Arc<dyn LightningTransactionStreamApi>>,
    handler: Arc<dyn LightningTransactionEventHandler>,
}

/// # LightningEventProcessor
///
/// A processor responsible for handling lightning network events from multiple lightning nodes.
///
/// ## Overview
///
/// `LightningEventProcessor` coordinates the subscription to lightning network event streams from multiple
/// nodes and processes the incoming events. It maintains the processing offset for each node
/// to enable resuming from the last processed event in case of service restart.
///
/// ## Features
///
/// - Subscribes to multiple lightning nodes simultaneously
/// - Resumes processing from the last known offset for each node
/// - Uses a channel-based approach for handling events
/// - Manages concurrent processing with tokio tasks
/// - Persists processing offsets to a PostgreSQL database
///
/// ## Architecture
///
/// The processor consists of:
/// - A collection of lightning node connections implementing `LightningStreamApi`
/// - An event handler implementing `LightningEventHandler`
/// - A PostgreSQL connection pool for persisting offsets
/// - A managed set of tasks for processing events
///
/// ## Examples
///
/// ### Creating and starting a LightningEventProcessor
///
/// ```no_run
/// use std::sync::Arc;
/// use sqlx::postgres::PgPoolOptions;
/// use payday_core::api::lightning_api::{LightningEventHandler, LightningStreamApi};
/// use payday::lightning_processor::LightningEventProcessor;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a PostgreSQL connection pool
///     let pool = PgPoolOptions::new()
///         .max_connections(5)
///         .connect("postgres://username:password@localhost/database_name").await?;
///
///     // Initialize your lightning nodes
///     let nodes: Vec<Arc<dyn LightningStreamApi>> = vec![];
///
///     // Initialize your event handler
///     let handler: Option<Arc<dyn LightningEventHandler>> = None;
///     
///     // Create the processor
///     let processor = LightningEventProcessor::new(
///         pool,
///         nodes,
///         handler.unwrap(),
///     );
///     
///     // Start the processor
///     let mut join_set = processor.start().await;
///     
///     // Wait for all tasks to complete or handle errors
///     while let Some(result) = join_set.join_next().await {
///         match result {
///             Ok(Ok(())) => println!("Task completed successfully"),
///             Ok(Err(e)) => println!("Task failed with error: {:?}", e),
///             Err(e) => println!("Failed to join task: {:?}", e),
///         }
///     }
///     
///     Ok(())
/// }
/// ```
impl LightningEventProcessor {
    pub fn new(
        pool: Pool<Postgres>,
        nodes: Vec<Arc<dyn LightningTransactionStreamApi>>,
        handler: Arc<dyn LightningTransactionEventHandler>,
    ) -> Self {
        Self {
            pool,
            nodes,
            handler,
        }
    }

    pub async fn start(&self) -> JoinSet<Result<(), JoinError>> {
        let (snd, mut rcv) = tokio::sync::mpsc::channel(100);
        let mut join_set = JoinSet::new();

        let offset_store = OffsetStore::new(
            self.pool.clone(),
            Some("payday.offsets".to_string()),
            Some("lightning".to_string()),
        );

        for node in &self.nodes {
            let last_offset = offset_store
                .get_offset(&node.node_id())
                .await
                .ok()
                .map(|o| o.offset);
            if let Ok(join) = node
                .subscribe_lightning_events(snd.clone(), last_offset)
                .await
            {
                join_set.spawn(join);
            } else {
                error!(
                    "Failed to subscribe to lightning events for node {}",
                    node.node_id()
                );
            }
        }

        let processor = LightningProcessor::new(Box::new(offset_store), self.handler.clone());
        let handle = tokio::spawn(async move {
            while let Some(event) = rcv.recv().await {
                info!("Received lightning event: {:?}", event);
                if let Err(err) = processor.process_event(event).await {
                    error!("Failed to process lightning event: {:?}", err);
                }
            }
        });

        join_set.spawn(handle);
        join_set
    }
}
