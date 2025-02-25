use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::payment::amount::Amount;

pub type InvoiceId = String;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidAmount(Amount),
    InvalidCurrency(String, String),
    ServiceError(String),
    InvoiceAlreadyExists(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidAmount(a) => write!(f, "Invoice invalid amount: {}", a),
            Error::InvalidCurrency(required, received) => write!(
                f,
                "Invoice invalid currency required: {} received: {}",
                required, received
            ),
            Error::ServiceError(err) => write!(f, "Invoice service error: {}", err),
            Error::InvoiceAlreadyExists(id) => write!(f, "Invoice already exists: {}", id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentType {
    BitcoinOnChain,
    BitcoinLightning,
    BitcoinUnified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: InvoiceId,
    pub node_id: String,
    pub payment_type: PaymentType,
    pub invoice_amount: Amount,
    pub received_amount: Amount,
    pub underpayment: bool,
    pub overpayment: bool,
    pub paid: bool,
    pub details: Option<PaymentDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaymentEvent {
    PaymentUnconfirmed(PaymentReceivedEventPayload),
    PaymentReceived(PaymentReceivedEventPayload),
    UnexpectedPaymentReceived(UnexpectedPaymentReceivedEventPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaymentDetails {
    OnChain(OnChainPaymentDetais),
    Lightning(LightningPaymentDetails),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainPaymentDetais {
    pub address: String,
    pub confirmations: u32,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningPaymentDetails {
    pub invoice: String,
    pub r_hash: String,
}

#[async_trait]
pub trait PaymentProcessorApi: Send + Sync {
    /// A unique name for this processor.
    fn name(&self) -> String;

    /// The payment type this processor supports.
    fn supported_payment_type(&self) -> PaymentType;

    /// Create an invoice.
    async fn create_invoice(
        &self,
        invoice_id: InvoiceId,
        amount: Amount,
        memo: Option<String>,
    ) -> crate::Result<Invoice>;

    /// Processes payment events for this system.
    async fn process_payment_events(&self) -> crate::Result<()>;
}
