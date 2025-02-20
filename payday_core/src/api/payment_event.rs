use crate::payment::amount::Amount;
use crate::payment::invoice::InvoiceId;
use crate::Result;

pub trait PaymentEventPublisherApi {
    /// Publishes a transaction event.
    fn publish_received_event(&self, event: PaymentReceivedEvent) -> Result<()>;
}

pub enum PaymentReceivedEvent {
    /// A payment for an invoice has been received but not confirmed yet.
    OnChainUnconfirmed(OnChainPaymentEvent),
    /// A payment for an invoice has been received and has been confirmed.
    OnChainConfirmed(OnChainPaymentEvent),
    /// We received a payment where there is no invoice in the system.
    OnChainUnexpected(OnChainUexpectedPaymentEvent),
    /// We received a payment for an invoice that has already been paid.
    OnChainUsedAddress(OnChainUsedAddressPaymentEvent),
}

pub struct OnChainData {
    pub node_id: String,
    pub address: String,
    pub confirmations: u64,
    pub transaction_id: Option<String>,
}

pub struct OnChainPaymentEvent {
    pub invoice_id: InvoiceId,
    pub invoice_amount: Amount,
    pub received_amount: Amount,
    pub underpayment: bool,
    pub overpayment: bool,
    pub on_chain_data: OnChainData,
    pub paid: bool,
}

pub struct OnChainUexpectedPaymentEvent {
    pub received_amount: Amount,
    pub on_chain_data: OnChainData,
    pub paid: bool,
}

pub struct OnChainUsedAddressPaymentEvent {
    pub received_amount: Amount,
    pub on_chain_data: OnChainData,
    pub original_invoice_id: InvoiceId,
    pub paid: bool,
}
