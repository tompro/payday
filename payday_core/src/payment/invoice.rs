use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};

use crate::payment::amount::Amount;

pub type InvoiceId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentType {
    BitcoinOnChain,
    BitcoinLightning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaymentEvent {
    PaymentUnconfirmed(PaymentReceivedEventPayload),
    PaymentReceived(PaymentReceivedEventPayload),
    UnexpectedPaymentReceived(UnexpectedPaymentReceivedEventPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReceivedEventPayload {
    pub invoice_id: InvoiceId,
    pub node_id: String,
    pub payment_type: PaymentType,
    pub invoice_amount: Amount,
    pub received_amount: Amount,
    pub underpayment: bool,
    pub overpayment: bool,
    pub paid: bool,
    pub details: PaymentDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnexpectedPaymentReceivedEventPayload {
    pub node_id: String,
    pub payment_type: PaymentType,
    pub received_amount: Amount,
    pub paid: bool,
    pub details: PaymentDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum PaymentDetails {
    OnChain(OnChainPaymentDetais),
    Lightning(LightningPaymentDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnChainPaymentDetais {
    pub address: String,
    pub confirmations: u32,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightningPaymentDetails {
    pub invoice: String,
    pub r_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvoiceDetails {
    OnChain(OnChainInvoiceDetails),
    Lightning(LightningInvoiceDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnChainInvoiceDetails {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightningInvoiceDetails {
    pub invoice: Bolt11Invoice,
}
