use crate::{payment::amount::Amount, Result};
use async_trait::async_trait;

#[async_trait]
pub trait GetLightningBalanceApi: Send + Sync {
    /// Get the current OnChain balance of the wallet.
    async fn get_onchain_balance(&self) -> Result<ChannelBalance>;
}

#[async_trait]
pub trait LightningInvoiceApi: Send + Sync {
    /// Get a new onchain address for the wallet.
    async fn create_invoice(&self) -> Result<LnInvoice>;
}

pub struct LnInvoice;

#[derive(Debug)]
pub struct ChannelBalance {
    pub local_balance: Amount,
    pub remote_balance: Amount,
}
