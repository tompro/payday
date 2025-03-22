use std::sync::Arc;

use payday_core::api::on_chain_api::{
    OnChainTransactionEventHandler, OnChainTransactionEventProcessorApi,
    OnChainTransactionStreamApi,
};
use payday_core::persistence::offset::OffsetStoreApi;
use payday_core::processor::on_chain_processor::OnChainTransactionProcessor;
use payday_postgres::offset::OffsetStore;
use sqlx::{Pool, Postgres};
use tokio::task::{JoinError, JoinSet};
use tracing::{error, info};

#[allow(dead_code)]
pub struct OnChainEventProcessor {
    pool: Pool<Postgres>,
    nodes: Vec<Arc<dyn OnChainTransactionStreamApi>>,
    handler: Arc<dyn OnChainTransactionEventHandler>,
}

/// # OnChainEventProcessor
///
/// A processor responsible for handling on-chain transaction events from multiple blockchain nodes.
///
/// ## Overview
///
/// `OnChainEventProcessor` coordinates the subscription to on-chain transaction streams from multiple
/// nodes and processes the incoming transaction events. It maintains the processing offset for each node
/// to enable resuming from the last processed block height in case of service restart.
///
/// ## Features
///
/// - Subscribes to multiple blockchain nodes simultaneously
/// - Resumes processing from the last known offset for each node
/// - Uses a channel-based approach for handling events
/// - Manages concurrent processing with tokio tasks
/// - Persists processing offsets to a PostgreSQL database
///
/// ## Architecture
///
/// The processor consists of:
/// - A collection of blockchain node connections implementing `OnChainTransactionStreamApi`
/// - An event handler implementing `OnChainTransactionEventHandler`
/// - A PostgreSQL connection pool for persisting offsets
/// - A managed set of tasks for processing events
///
/// ## Examples
///
/// ### Creating and starting an OnChainEventProcessor
///
/// ```no_run
/// use std::sync::Arc;
/// use sqlx::postgres::PgPoolOptions;
/// use payday_core::api::on_chain_api::{OnChainTransactionEventHandler, OnChainTransactionStreamApi};
/// use payday::on_chain_processor::OnChainEventProcessor;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a PostgreSQL connection pool
///     let pool = PgPoolOptions::new()
///         .max_connections(5)
///         .connect("postgres://username:password@localhost/database_name").await?;
///
///     // Initialize your blockchain nodes
///     let nodes: Vec<Arc<dyn OnChainTransactionStreamApi>> = vec![];
///
///     // Initialize your event handler
///     let handler: Option<Arc<dyn OnChainTransactionEventHandler>> = None;
///     
///     // Create the processor
///     let processor = OnChainEventProcessor::new(
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
impl OnChainEventProcessor {
    pub fn new(
        pool: Pool<Postgres>,
        nodes: Vec<Arc<dyn OnChainTransactionStreamApi>>,
        handler: Arc<dyn OnChainTransactionEventHandler>,
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
            Some("on_chain".to_string()),
        );

        for node in &self.nodes {
            let start_height: Option<u64> = offset_store
                .get_offset(&node.node_id())
                .await
                .ok()
                .map(|o| o.offset);
            if let Ok(join) = node
                .subscribe_on_chain_transactions(snd.clone(), start_height)
                .await
            {
                join_set.spawn(join);
            } else {
                error!(
                    "Failed to subscribe to on chain transactions for node {}",
                    node.node_id()
                );
            }
        }

        let processor =
            OnChainTransactionProcessor::new(Box::new(offset_store), self.handler.clone());
        let handle = tokio::spawn(async move {
            while let Some(event) = rcv.recv().await {
                info!("Received event: {:?}", event);
                if let Err(err) = processor.process_event(event).await {
                    error!("Failed to process on chain event: {:?}", err);
                }
            }
        });

        join_set.spawn(handle);
        join_set
    }
}
