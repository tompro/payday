use std::collections::HashMap;

use async_trait::async_trait;
use bitcoin::{Address, Amount};
use payday_core::PaydayResult;
use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use crate::on_chain_processor::OnChainTransactionEvent;

#[async_trait]
pub trait OnChainApi: Send + Sync {
    /// Get the current balances (onchain and lightning) of the wallet.
    async fn get_balance(&self) -> PaydayResult<Balance>;

    /// Get a new onchain address for the wallet.
    async fn new_address(&self) -> PaydayResult<Address>;

    /// Get history of onchain transactions between start_height and end_height.
    async fn get_onchain_transactions(
        &self,
        start_height: i32,
        end_height: i32,
    ) -> PaydayResult<Vec<OnChainTransactionEvent>>;

    /// Given an onchain address string, parses and validates that it is a valid
    /// address for this nodes network.
    fn validate_address(&self, address: &str) -> PaydayResult<Address>;

    /// Estimate the fee for a transaction.
    async fn estimate_fee(
        &self,
        target_conf: i32,
        outputs: HashMap<String, Amount>,
    ) -> PaydayResult<Amount>;

    /// Send coins to an address.
    async fn send(
        &self,
        amount: Amount,
        address: String,
        sats_per_vbyte: Amount,
    ) -> PaydayResult<OnChainPaymentResult>;

    /// Send coins to multiple addresses.
    async fn batch_send(
        &self,
        outputs: HashMap<String, Amount>,
        sats_per_vbyte: Amount,
    ) -> PaydayResult<OnChainPaymentResult>;
}

#[async_trait]
pub trait OnChainStreamApi: Send + Sync {
    fn process_events(&self) -> PaydayResult<JoinHandle<()>>;
}

#[async_trait]
pub trait OnChainTransactionStreamSubscriber: Send + Sync {
    fn subscribe_events(&self) -> PaydayResult<Receiver<OnChainTransactionEvent>>;
}

#[derive(Debug)]
pub struct OnChainBalance {
    pub total_balance: Amount,
    pub unconfirmed_balance: Amount,
    pub confirmed_balance: Amount,
}

#[derive(Debug)]
pub struct ChannelBalance {
    pub local_balance: Amount,
    pub remote_balance: Amount,
}

#[derive(Debug)]
pub struct Balance {
    pub onchain: OnChainBalance,
    pub channel: ChannelBalance,
}

#[derive(Debug)]
pub struct OnChainPaymentResult {
    pub tx_id: String,
    pub amounts: HashMap<String, Amount>,
    pub fee: Amount,
}
