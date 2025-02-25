use async_trait::async_trait;

use crate::payment::{
    invoice::{InvoiceDetails, InvoiceId, PaymentType},
    Amount, Result,
};

#[async_trait]
pub trait InvoiceServiceApi: Send + Sync {
    async fn create_invoice(
        &self,
        invoice_id: InvoiceId,
        node_id: String,
        payment_type: PaymentType,
        amount: Amount,
        memo: Option<String>,
    ) -> Result<InvoiceDetails>;
}
