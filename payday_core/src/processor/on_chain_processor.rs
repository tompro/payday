use std::sync::Arc;

use crate::{
    api::on_chain_api::{
        OnChainTransactionEvent, OnChainTransactionEventHandler,
        OnChainTransactionEventProcessorApi,
    },
    persistence::offset::OffsetStoreApi,
    Result,
};
use async_trait::async_trait;

pub struct OnChainTransactionProcessor {
    block_height_store: Box<dyn OffsetStoreApi>,
    handler: Arc<dyn OnChainTransactionEventHandler>,
}

impl OnChainTransactionProcessor {
    pub fn new(
        block_height_store: Box<dyn OffsetStoreApi>,
        handler: Arc<dyn OnChainTransactionEventHandler>,
    ) -> Self {
        Self {
            block_height_store,
            handler,
        }
    }
}

#[async_trait]
impl OnChainTransactionEventProcessorApi for OnChainTransactionProcessor {
    async fn get_offset(&self, id: &str) -> Result<u64> {
        self.block_height_store
            .get_offset(id)
            .await
            .map(|o| o.offset)
    }

    async fn set_block_height(&self, id: &str, block_height: u64) -> Result<()> {
        self.block_height_store.set_offset(id, block_height).await
    }

    async fn process_event(&self, event: OnChainTransactionEvent) -> Result<()> {
        let block_height = event.block_height();
        let node_id = event.node_id();
        self.handler.process_event(event).await?;
        if let Some(bh) = block_height {
            self.set_block_height(&node_id, bh as u64).await?;
        }
        Ok(())
    }
}

pub struct OnChainTransactionPrintHandler;

#[async_trait]
impl OnChainTransactionEventHandler for OnChainTransactionPrintHandler {
    async fn process_event(&self, event: OnChainTransactionEvent) -> Result<()> {
        println!("OnChainEventTransactionEvent: {:?}", event);
        Ok(())
    }
}
