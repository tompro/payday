use std::sync::Arc;

use payday_core::api::on_chain_api::{OnChainTransactionEventHandler, OnChainTransactionStreamApi};

#[allow(dead_code)]
pub struct OnChainEventHandler {
    nodes: Vec<Arc<dyn OnChainTransactionStreamApi>>,
    handler: Arc<dyn OnChainTransactionEventHandler>,
}

impl OnChainEventHandler {
    pub fn new(
        nodes: Vec<Arc<dyn OnChainTransactionStreamApi>>,
        handler: Arc<dyn OnChainTransactionEventHandler>,
    ) -> Self {
        Self { nodes, handler }
    }

    pub async fn process_events(&self) {}
}
