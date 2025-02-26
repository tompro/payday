use async_trait::async_trait;
use payday_core::{
    Result,
    aggregate::on_chain_aggregate::{OnChainCommand, OnChainInvoice},
    api::on_chain_api::{OnChainTransactionEvent, OnChainTransactionEventHandler},
    persistence::cqrs::Cqrs,
};
use postgres_es::PostgresEventRepository;
use tracing::{error, info};

pub struct OnChainService {
    aggregate: Cqrs<OnChainInvoice, PostgresEventRepository>,
}

impl OnChainService {
    pub fn new(aggregate: Cqrs<OnChainInvoice, PostgresEventRepository>) -> Self {
        Self { aggregate }
    }
}

#[async_trait]
impl OnChainTransactionEventHandler for OnChainService {
    async fn process_event(&self, event: OnChainTransactionEvent) -> Result<()> {
        info!("Received on-chain event: {:?}", event);
        let command: OnChainCommand = event.into();
        info!("Executing on-chain command: {:?}", command);
        if let Err(res) = self.aggregate.execute(&command.id, command.command).await {
            error!("Failed to execute on-chain command with: {:?}", res);
        } else {
            info!("Successfully executed on-chain command");
        }
        Ok(())
    }
}
