use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{payment::amount::Amount, PaydayResult};

pub type InvoiceId = String;
pub type PaymentType = String;
pub type InvoiceResult<T> = Result<T, InvoiceError>;

#[derive(Debug, Clone)]
pub enum InvoiceError {
    InvalidAmount(Amount),
    InvalidCurrency(String, String),
    ServiceError(String),
}

impl std::error::Error for InvoiceError {}

impl Display for InvoiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceError::InvalidAmount(a) => write!(f, "Invoice invalid amount: {}", a),
            InvoiceError::InvalidCurrency(required, received) => write!(
                f,
                "Invoice invalid currency required: {} received: {}",
                required, received
            ),
            InvoiceError::ServiceError(err) => write!(f, "Invoice service error: {}", err),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub service_name: String,
    pub invoice_id: InvoiceId,
    pub amount: Amount,
    pub payment_type: PaymentType,
    pub payment_info: Value,
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
    ) -> PaydayResult<Invoice>;

    /// Processes payment events for this system.
    async fn process_payment_events(&self) -> PaydayResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LnInvoice {
    pub invoice: String,
    pub r_hash: String,
    pub add_index: u64,
}
