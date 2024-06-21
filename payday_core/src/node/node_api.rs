use async_trait::async_trait;
use bitcoin::{Address, Amount};

use crate::error::PaydayResult;

#[async_trait]
pub trait NodeApi {
    /// Get the current balances (onchain and lightning) of the wallet.
    async fn get_balance(&mut self) -> PaydayResult<Balance>;

    /// Get a new onchain address for the wallet.
    async fn new_address(&mut self) -> PaydayResult<Address>;

    // async fn estimate_fee(&self, target_conf: u8, addr_to_amount: HashMap<String, u64>) -> u64;
    // async fn send_coins(&self, amount: u64, address: String) -> u64;
    // async fn subscribe_transactions(&self, start_height: u64) -> Response<Streaming<Transaction>>;
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
