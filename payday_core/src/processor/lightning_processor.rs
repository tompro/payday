use crate::{
    api::lightining_api::{
        LightningTransactionEvent, LightningTransactionEventHandler,
        LightningTransactionEventProcessorApi,
    },
    persistence::offset::OffsetStoreApi,
    Result,
};
use async_trait::async_trait;

pub struct LightningTransactionProcessor {
    node_id: String,
    settle_index_store: Box<dyn OffsetStoreApi>,
    handler: Box<dyn LightningTransactionEventHandler>,
}

impl LightningTransactionProcessor {
    pub fn new(
        node_id: &str,
        settle_index_store: Box<dyn OffsetStoreApi>,
        handler: Box<dyn LightningTransactionEventHandler>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            settle_index_store,
            handler,
        }
    }
}

#[async_trait]
impl LightningTransactionEventProcessorApi for LightningTransactionProcessor {
    fn node_id(&self) -> String {
        self.node_id.to_string()
    }
    async fn get_offset(&self) -> Result<u64> {
        self.settle_index_store.get_offset().await.map(|o| o.offset)
    }

    async fn set_offset(&self, block_height: u64) -> Result<()> {
        self.settle_index_store.set_offset(block_height).await
    }

    async fn process_event(&self, event: LightningTransactionEvent) -> Result<()> {
        let index = event.settle_index();
        self.handler.process_event(event).await?;
        if let Some(idx) = index {
            self.set_offset(idx).await?;
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
