use async_trait::async_trait;

use crate::{
    payment::{
        invoice::{InvoiceDetails, PaymentType},
        Amount,
    },
    Result,
};

#[async_trait]
pub trait NodeApi: Send + Sync {
    fn node_id(&self) -> String;
    fn supports_payment_types(&self, payment_type: PaymentType) -> bool;
    async fn create_invoice(
        &self,
        payment_type: PaymentType,
        amount: Amount,
        memo: Option<String>,
        ttl: Option<u64>,
    ) -> Result<InvoiceDetails>;
}
