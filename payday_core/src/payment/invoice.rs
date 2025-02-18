use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::payment::amount::Amount;

pub type InvoiceId = String;
pub type PaymentType = String;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidAmount(Amount),
    InvalidCurrency(String, String),
    ServiceError(String),
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
    ) -> crate::Result<Invoice>;

    /// Processes payment events for this system.
    async fn process_payment_events(&self) -> crate::Result<()>;
}
