use std::sync::Arc;

use async_trait::async_trait;
use payday_core::{
    api::on_chain_api::{
        OnChainTransactionEvent, OnChainTransactionEventHandler,
        OnChainTransactionEventProcessorApi,
    },
    persistence::block_height::BlockHeightStoreApi,
    Result,
};
use tokio::sync::Mutex;

pub struct OnChainTransactionProcessor {
    node_id: String,
    block_height_store: Box<dyn BlockHeightStoreApi>,
    handler: Box<dyn OnChainTransactionEventHandler>,
    current_block_height: Arc<Mutex<i32>>,
}

impl OnChainTransactionProcessor {
    pub fn new(
        node_id: &str,
        block_height_store: Box<dyn BlockHeightStoreApi>,
        handler: Box<dyn OnChainTransactionEventHandler>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            block_height_store,
            handler,
            current_block_height: Arc::new(Mutex::new(-1)),
        }
    }
}

#[async_trait]
impl OnChainTransactionEventProcessorApi for OnChainTransactionProcessor {
    fn node_id(&self) -> String {
        self.node_id.to_string()
    }
    async fn get_block_height(&self) -> Result<i32> {
        let mut current_block_height = self.current_block_height.lock().await;
        if *current_block_height < 0 {
            *current_block_height = self
                .block_height_store
                .get_block_height(&self.node_id)
                .await?
                .block_height as i32;
        }
        Ok(*current_block_height)
    }
    async fn set_block_height(&self, block_height: i32) -> Result<()> {
        let mut current_block_height = self.current_block_height.lock().await;
        if *current_block_height < block_height {
            self.block_height_store
                .set_block_height(&self.node_id, block_height as u64)
                .await?;
            *current_block_height = block_height;
        }
        Ok(())
    }
    async fn process_event(&self, event: OnChainTransactionEvent) -> Result<()> {
        let block_height = event.block_height();
        self.handler.process_event(event).await?;
        if let Some(bh) = block_height {
            self.set_block_height(bh).await?;
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
