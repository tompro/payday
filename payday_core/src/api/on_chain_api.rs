use std::collections::HashMap;

use crate::Result;
use async_trait::async_trait;
use bitcoin::{Address, Amount};
use tokio::{sync::mpsc::Sender, task::JoinHandle};

#[async_trait]
pub trait GetOnChainBalanceApi: Send + Sync {
    /// Get the current OnChain balance of the wallet.
    async fn get_onchain_balance(&self) -> Result<OnChainBalance>;
}

#[async_trait]
pub trait OnChainInvoiceApi: Send + Sync {
    /// Get a new onchain address for the wallet.
    async fn new_address(&self) -> Result<Address>;
}

#[async_trait]
pub trait OnChainPaymentApi: Send + Sync {
    /// Given an onchain address string, parses and validates that it is a valid
    /// address for this nodes network.
    fn validate_address(&self, address: &str) -> Result<Address>;

    /// Estimate the fee for a transaction.
    async fn estimate_fee(
        &self,
        target_conf: i32,
        outputs: HashMap<String, Amount>,
    ) -> Result<Amount>;

    /// Send coins to an address.
    async fn send(
        &self,
        amount: Amount,
        address: String,
        sats_per_vbyte: Amount,
    ) -> Result<OnChainPaymentResult>;

    /// Send coins to multiple addresses.
    async fn batch_send(
        &self,
        outputs: HashMap<String, Amount>,
        sats_per_vbyte: Amount,
    ) -> Result<OnChainPaymentResult>;
}

#[async_trait]
pub trait OnChainTransactionApi: Send + Sync {
    /// Get history of onchain transactions between start_height and end_height.
    async fn get_onchain_transactions(
        &self,
        start_height: i32,
        end_height: i32,
    ) -> Result<Vec<OnChainTransactionEvent>>;
}

#[async_trait]
pub trait OnChainTransactionEventProcessorApi: Send + Sync {
    fn node_id(&self) -> String;
    async fn get_offset(&self) -> Result<u64>;
    async fn set_block_height(&self, block_height: u64) -> Result<()>;
    async fn process_event(&self, event: OnChainTransactionEvent) -> Result<()>;
}

#[async_trait]
pub trait OnChainTransactionEventHandler: Send + Sync {
    async fn process_event(&self, event: OnChainTransactionEvent) -> Result<()>;
}

#[async_trait]
pub trait OnChainTransactionStreamApi: Send + Sync {
    async fn subscribe_on_chain_transactions(
        &self,
        sender: Sender<OnChainTransactionEvent>,
        start_height: Option<i32>,
    ) -> Result<JoinHandle<()>>;
}

#[derive(Debug, Clone)]
pub struct OnChainBalance {
    pub total_balance: Amount,
    pub unconfirmed_balance: Amount,
    pub confirmed_balance: Amount,
}

#[derive(Debug, Clone)]
pub struct OnChainPaymentResult {
    pub tx_id: String,
    pub amounts: HashMap<String, Amount>,
    pub fee: Amount,
}

#[derive(Debug, Clone)]
pub enum OnChainTransactionEvent {
    ReceivedUnconfirmed(OnChainTransaction),
    ReceivedConfirmed(OnChainTransaction),
    SentUnconfirmed(OnChainTransaction),
    SentConfirmed(OnChainTransaction),
}

impl OnChainTransactionEvent {
    pub fn block_height(&self) -> Option<i32> {
        match self {
            OnChainTransactionEvent::ReceivedConfirmed(tx) => Some(tx.block_height),
            OnChainTransactionEvent::SentConfirmed(tx) => Some(tx.block_height),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OnChainTransaction {
    pub tx_id: String,
    pub block_height: i32,
    pub node_id: String,
    pub address: Address,
    pub amount: Amount,
    pub confirmations: i32,
}
