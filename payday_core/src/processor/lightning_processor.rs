use crate::{
    api::lightning_api::{
        LightningTransactionEvent, LightningTransactionEventHandler,
        LightningTransactionEventProcessorApi,
    },
    persistence::offset::OffsetStoreApi,
    Result,
};
use async_trait::async_trait;

pub struct LightningTransactionProcessor {
    settle_index_store: Box<dyn OffsetStoreApi>,
    handler: Box<dyn LightningTransactionEventHandler>,
}

impl LightningTransactionProcessor {
    pub fn new(
        settle_index_store: Box<dyn OffsetStoreApi>,
        handler: Box<dyn LightningTransactionEventHandler>,
    ) -> Self {
        Self {
            settle_index_store,
            handler,
        }
    }
}

#[async_trait]
impl LightningTransactionEventProcessorApi for LightningTransactionProcessor {
    async fn get_offset(&self, id: &str) -> Result<u64> {
        self.settle_index_store
            .get_offset(id)
            .await
            .map(|o| o.offset)
    }

    async fn set_offset(&self, id: &str, block_height: u64) -> Result<()> {
        self.settle_index_store.set_offset(id, block_height).await
    }

    async fn process_event(&self, event: LightningTransactionEvent) -> Result<()> {
        let index = event.settle_index();
        let node_id = event.node_id();
        self.handler.process_event(event).await?;
        if let Some(idx) = index {
            self.set_offset(&node_id, idx).await?;
        }
        Ok(())
    }
}

pub struct LightningTransactionPrintHandler;

#[async_trait]
impl LightningTransactionEventHandler for LightningTransactionPrintHandler {
    async fn process_event(&self, event: LightningTransactionEvent) -> Result<()> {
        println!("LightningTransactionEvent: {:?}", event);
        Ok(())
    }
}
