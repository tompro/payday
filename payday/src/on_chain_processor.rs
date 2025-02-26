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
