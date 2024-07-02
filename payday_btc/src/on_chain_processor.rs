use async_trait::async_trait;
use bitcoin::{Address, Amount};
use payday_core::PaydayResult;

#[async_trait]
pub trait OnChainTransactionEventProcessor: Send + Sync {
    async fn get_block_height(&self) -> PaydayResult<i32>;
    async fn process_event(&self, event: OnChainTransactionEvent) -> PaydayResult<()>;
}

pub struct OnChainTransactionEventPrinter;

#[async_trait]
impl OnChainTransactionEventProcessor for OnChainTransactionEventPrinter {
    async fn get_block_height(&self) -> PaydayResult<i32> {
        Ok(0)
    }

    async fn process_event(&self, event: OnChainTransactionEvent) -> PaydayResult<()> {
        println!("{:?}", event);
        Ok(())
    }
}

#[derive(Debug)]
pub enum OnChainTransactionEvent {
    ReceivedUnconfirmed(OnChainTransaction),
    ReceivedConfirmed(OnChainTransaction),
    SentUnconfirmed(OnChainTransaction),
    SentConfirmed(OnChainTransaction),
}

#[derive(Debug, Clone)]
pub struct OnChainTransaction {
    pub tx_id: String,
    pub block_height: i32,
    pub address: Address,
    pub amount: Amount,
    pub confirmations: i32,
}
