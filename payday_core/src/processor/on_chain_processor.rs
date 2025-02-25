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
    node_id: String,
    block_height_store: Box<dyn OffsetStoreApi>,
    handler: Box<dyn OnChainTransactionEventHandler>,
}

impl OnChainTransactionProcessor {
    pub fn new(
        node_id: &str,
        block_height_store: Box<dyn OffsetStoreApi>,
        handler: Box<dyn OnChainTransactionEventHandler>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            block_height_store,
            handler,
        }
    }
}

#[async_trait]
impl OnChainTransactionEventProcessorApi for OnChainTransactionProcessor {
    fn node_id(&self) -> String {
        self.node_id.to_string()
    }

    async fn get_offset(&self) -> Result<u64> {
        self.block_height_store.get_offset().await.map(|o| o.offset)
    }

    async fn set_block_height(&self, block_height: u64) -> Result<()> {
        self.block_height_store.set_offset(block_height).await
    }

    async fn process_event(&self, event: OnChainTransactionEvent) -> Result<()> {
        let block_height = event.block_height();
        self.handler.process_event(event).await?;
        if let Some(bh) = block_height {
            self.set_block_height(bh as u64).await?;
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
