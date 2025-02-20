use crate::{
    api::lightining_api::LightningTransactionEvent,
    payment::{
        amount::Amount,
        currency::Currency,
        invoice::{Error, InvoiceId},
    },
};
use async_trait::async_trait;
use cqrs_es::{Aggregate, DomainEvent};
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LightningInvoice {
    pub invoice_id: InvoiceId,
    pub node_id: String,
    pub r_hash: String,
    pub invoice: String,
    pub amount: Amount,
    pub received_amount: Amount,
    pub overpaid: bool,
    pub paid: bool,
}

#[async_trait]
pub trait LightningInvoiceService: Send + Sync {}

pub struct LightningInvoiceServices {}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Deserialize)]
pub enum LightningInvoiceCommand {
    CreateInvoice {
        invoice_id: InvoiceId,
        node_id: String,
        amount: Amount,
        invoice: Bolt11Invoice,
    },
    SettleInvoice {
        received_amount: Amount,
    },
}

impl From<LightningTransactionEvent> for LightningInvoiceCommand {
    fn from(event: LightningTransactionEvent) -> Self {
        match event {
            LightningTransactionEvent::Settled(tx) => LightningInvoiceCommand::SettleInvoice {
                received_amount: tx.amount_paid,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LightningInvoiceEvent {
    InvoiceCreated {
        invoice_id: InvoiceId,
        node_id: String,
        r_hash: String,
        amount: Amount,
        invoice: String,
    },
    InvoiceSettled {
        received_amount: Amount,
        overpaid: bool,
        paid: bool,
    },
}

impl DomainEvent for LightningInvoiceEvent {
    fn event_type(&self) -> String {
        let event_type = match self {
            LightningInvoiceEvent::InvoiceCreated { .. } => "LightningInvoiceCreated",
            LightningInvoiceEvent::InvoiceSettled { .. } => "LightningInvoiceSettled",
        };
        event_type.to_string()
    }

    fn event_version(&self) -> String {
        "1.0.0".to_string()
    }
}

#[async_trait]
impl Aggregate for LightningInvoice {
    type Command = LightningInvoiceCommand;
    type Event = LightningInvoiceEvent;
    type Error = Error;
    type Services = ();

    fn aggregate_type() -> String {
        "LightningInvoice".to_string()
    }

    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            LightningInvoiceCommand::CreateInvoice {
                invoice_id,
                node_id,
                amount,
                invoice,
            } => {
                if amount.currency != Currency::Btc {
                    return Err(Error::InvalidCurrency(
                        amount.currency.to_string(),
                        Currency::Btc.to_string(),
                    ));
                }

                let r_hash = invoice.payment_hash().to_string();
                let invoice = invoice.to_string();

                Ok(vec![LightningInvoiceEvent::InvoiceCreated {
                    invoice_id,
                    node_id,
                    r_hash,
                    amount,
                    invoice,
                }])
            }
            LightningInvoiceCommand::SettleInvoice { received_amount } => {
                Ok(vec![LightningInvoiceEvent::InvoiceSettled {
                    received_amount,
                    overpaid: received_amount.amount > self.amount.amount,
                    paid: received_amount.amount >= self.amount.amount,
                }])
            }
        }
    }

    fn apply(&mut self, event: Self::Event) {
        match event {
            LightningInvoiceEvent::InvoiceCreated {
                invoice_id,
                node_id,
                r_hash,
                amount,
                invoice,
            } => {
                self.invoice_id = invoice_id;
                self.node_id = node_id;
                self.r_hash = r_hash;
                self.amount = amount;
                self.invoice = invoice;
            }
            LightningInvoiceEvent::InvoiceSettled {
                received_amount,
                overpaid,
                paid,
            } => {
                self.received_amount = received_amount;
                self.overpaid = overpaid;
                self.paid = paid;
            }
        }
    }
}
