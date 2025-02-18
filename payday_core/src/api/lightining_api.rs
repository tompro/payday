use crate::{payment::amount::Amount, Result};
use async_trait::async_trait;
use lightning_invoice::Bolt11Invoice;

use super::on_chain_api::OnChainBalance;

#[async_trait]
pub trait GetLightningBalanceApi: Send + Sync {
    /// Get the current OnChain balance of the wallet.
    async fn get_channel_balance(&self) -> Result<ChannelBalance>;

    /// Get the current OnChain and channel balances of the Lightning wallet.
    async fn get_balances(&self) -> Result<NodeBalance>;
}

#[async_trait]
pub trait LightningInvoiceApi: Send + Sync {
    /// Get a new onchain address for the wallet.
    async fn create_ln_invoice(
        &self,
        amount: Amount,
        memo: Option<String>,
        ttl: Option<i64>,
    ) -> Result<LnInvoice>;
}

#[async_trait]
pub trait LightningPaymentApi: Send + Sync {
    /// Pays a given BOLT11 invoice.
    async fn pay_invoice(&self, invoice: Bolt11Invoice) -> Result<()>;

    /// Pays the given amount to the given node public key.
    async fn pay_to_node_pub_key(&self, pub_key: String, amount: Amount) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct LnInvoice {
    pub invoice: String,
    pub r_hash: String,
    pub add_index: u64,
}

#[derive(Debug, Clone)]
pub struct ChannelBalance {
    pub local_balance: Amount,
    pub remote_balance: Amount,
}

/// Lightning nodes have an onchain and channel balance.
#[derive(Debug, Clone)]
pub struct NodeBalance {
    pub onchain: OnChainBalance,
    pub channel: ChannelBalance,
}
