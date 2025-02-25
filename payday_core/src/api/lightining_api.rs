use std::fmt::Display;

use crate::{payment::amount::Amount, Error, Result};
use async_trait::async_trait;
use lightning_invoice::Bolt11Invoice;
use tokio::{sync::mpsc::Sender, task::JoinHandle};

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

/// Allows consuming Lightning transaction events from a Lightning node.
#[async_trait]
pub trait LightningTransactionStreamApi: Send + Sync {
    /// Subscribe to Lightning transaction events. The receiver of the channel will get
    /// LightningTransaction events. The subscription will resume the node event stream
    /// from the given settle_index. At the moment only settled transactions are populated.
    async fn subscribe_lightning_transactions(
        &self,
        sender: Sender<LightningTransactionEvent>,
        settle_index: Option<u64>,
    ) -> Result<JoinHandle<()>>;
}

#[async_trait]
pub trait LightningTransactionEventProcessorApi: Send + Sync {
    fn node_id(&self) -> String;
    async fn get_offset(&self) -> Result<u64>;
    async fn set_offset(&self, settle_index: u64) -> Result<()>;
    async fn process_event(&self, event: LightningTransactionEvent) -> Result<()>;
}

#[async_trait]
pub trait LightningTransactionEventHandler: Send + Sync {
    async fn process_event(&self, event: LightningTransactionEvent) -> Result<()>;
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

#[derive(Debug, Clone)]
pub struct LightningTransaction {
    pub node_id: String,
    pub r_hash: String,
    pub invoice: String,
    pub amount: Amount,
    pub amount_paid: Amount,
    pub settle_index: u64,
}

#[derive(Debug, Clone)]
pub enum LightningTransactionEvent {
    Settled(LightningTransaction),
}

impl LightningTransactionEvent {
    pub fn settle_index(&self) -> Option<u64> {
        match self {
            LightningTransactionEvent::Settled(tx) => Some(tx.settle_index),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InvoiceState {
    OPEN,
    SETTLED,
    CANCELED,
    ACCEPTED,
}

impl TryFrom<i32> for InvoiceState {
    type Error = Error;
    fn try_from(value: i32) -> Result<Self> {
        match value {
            0 => Ok(InvoiceState::OPEN),
            1 => Ok(InvoiceState::SETTLED),
            2 => Ok(InvoiceState::CANCELED),
            3 => Ok(InvoiceState::ACCEPTED),
            _ => Err(Error::InvalidInvoiceState(format!(
                "Invalid invoice state: {}",
                value
            ))),
        }
    }
}

// In this direction we don't need to check for invalid values.
#[allow(clippy::from_over_into)]
impl Into<i32> for InvoiceState {
    fn into(self) -> i32 {
        match self {
            InvoiceState::OPEN => 0,
            InvoiceState::SETTLED => 1,
            InvoiceState::CANCELED => 2,
            InvoiceState::ACCEPTED => 3,
        }
    }
}

impl Display for InvoiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
