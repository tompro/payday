use std::sync::Arc;

use async_trait::async_trait;
use cqrs_es::{Aggregate, DomainEvent};
use serde::{Deserialize, Serialize};

use crate::{
    api::invoice_api::InvoiceServiceApi,
    payment::{
        invoice::{InvoiceDetails, InvoiceId, PaymentDetails, PaymentType},
        Amount, Error,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Invoice {
    pub invoice_id: InvoiceId,
    pub node_id: String,
    pub payment_types: Vec<PaymentType>,
    pub invoice_amount: Amount,
    pub received_amount: Amount,
    pub underpayment: bool,
    pub overpayment: bool,
    pub paid: bool,
    pub invoice_details: Vec<InvoiceDetails>,
    pub details: Option<PaymentDetails>,
    pub used_payment_type: Option<PaymentType>,
}

#[derive(Debug, Deserialize)]
pub enum InvoiceCommand {
    CreateInvoice {
        invoice_id: InvoiceId,
        node_id: String,
        amount: Amount,
        memo: Option<String>,
        payment_types: Vec<PaymentType>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvoiceEvent {
    Created {
        invoice_id: InvoiceId,
        node_id: String,
        amount: Amount,
        payment_types: Vec<PaymentType>,
        invoice_details: Vec<InvoiceDetails>,
    },
    Paid {
        payment_type: PaymentType,
        received_amount: Amount,
        underpayment: bool,
        overpayment: bool,
        paid: bool,
        details: Option<PaymentDetails>,
    },
}

impl DomainEvent for InvoiceEvent {
    fn event_type(&self) -> String {
        let event_type = match self {
            InvoiceEvent::Created { .. } => "InvoiceCreated",
            InvoiceEvent::Paid { .. } => "InvoicePaid",
        };
        event_type.to_string()
    }

    fn event_version(&self) -> String {
        "1.0.0".to_string()
    }
}

#[async_trait]
impl Aggregate for Invoice {
    type Command = InvoiceCommand;
    type Event = InvoiceEvent;
    type Error = Error;
    type Services = Arc<dyn InvoiceServiceApi>;

    fn aggregate_type() -> String {
        "Invoice".to_string()
    }

    async fn handle(
        &self,
        command: Self::Command,
        services: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            InvoiceCommand::CreateInvoice {
                invoice_id,
                node_id,
                amount,
                memo,
                payment_types,
            } => {
                let mut invoice_details = vec![];
                for tpe in payment_types.clone() {
                    if let Ok(details) = services
                        .create_invoice(
                            invoice_id.clone(),
                            node_id.clone(),
                            tpe.clone(),
                            amount,
                            memo.clone(),
                        )
                        .await
                    {
                        invoice_details.push(details);
                    }
                }

                // here we can add failover if a node can not produce invoices
                if invoice_details.is_empty() {
                    return Err(Error::InvoiceDetailsCreation(format!(
                        "Could not create any invoices on node {node_id}"
                    )));
                }

                Ok(vec![InvoiceEvent::Created {
                    invoice_id,
                    node_id,
                    amount,
                    payment_types,
                    invoice_details,
                }])
            }
        }
    }

    fn apply(&mut self, event: Self::Event) {
        match event {
            InvoiceEvent::Created {
                invoice_id,
                node_id,
                amount,
                payment_types,
                invoice_details,
            } => {
                self.invoice_id = invoice_id;
                self.node_id = node_id;
                self.invoice_amount = amount;
                self.payment_types = payment_types;
                self.invoice_details = invoice_details;
            }
            InvoiceEvent::Paid {
                payment_type,
                received_amount,
                underpayment,
                overpayment,
                paid,
                details,
            } => {
                self.received_amount = received_amount;
                self.underpayment = underpayment;
                self.overpayment = overpayment;
                self.paid = paid;
                self.details = details;
                self.used_payment_type = Some(payment_type);
            }
        }
    }
}
