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
                if !self.invoice_id.is_empty() {
                    return Err(Error::InvoiceAlreadyExists(invoice_id));
                }

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
                if self.paid {
                    return Ok(vec![]);
                }
                Ok(vec![LightningInvoiceEvent::InvoiceSettled {
                    received_amount,
                    overpaid: received_amount.cent_amount > self.amount.cent_amount,
                    paid: received_amount.cent_amount >= self.amount.cent_amount,
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cqrs_es::test::TestFramework;

    use super::*;

    type LightningInvoiceTestFramework = TestFramework<LightningInvoice>;

    #[test]
    fn test_create_lightning_invoice() {
        let expected_event = mock_created_event();
        LightningInvoiceTestFramework::with(())
            .given_no_previous_events()
            .when(LightningInvoiceCommand::CreateInvoice {
                invoice_id: "123".to_string(),
                node_id: "node1".to_string(),
                amount: Amount::sats(100_000),
                invoice: get_invoice(),
            })
            .then_expect_events(vec![expected_event]);
    }

    #[test]
    fn test_settle_lightning_invoice() {
        let expected_event = mock_settled_event(Amount::sats(100_000), false, true);
        LightningInvoiceTestFramework::with(())
            .given(vec![mock_created_event()])
            .when(LightningInvoiceCommand::SettleInvoice {
                received_amount: Amount::sats(100_000),
            })
            .then_expect_events(vec![expected_event]);
    }

    #[test]
    fn test_create_lightning_invoice_invalid_currency() {
        let expected_error = Error::InvalidCurrency("USD".to_string(), "BTC".to_string());
        LightningInvoiceTestFramework::with(())
            .given_no_previous_events()
            .when(LightningInvoiceCommand::CreateInvoice {
                invoice_id: "123".to_string(),
                node_id: "node1".to_string(),
                amount: Amount::new(Currency::Usd, 100_000),
                invoice: get_invoice(),
            })
            .then_expect_error(expected_error);
    }

    #[test]
    fn test_settle_lightning_invoice_overpaid() {
        let expected_event = mock_settled_event(Amount::sats(200_000), true, true);
        LightningInvoiceTestFramework::with(())
            .given(vec![mock_created_event()])
            .when(LightningInvoiceCommand::SettleInvoice {
                received_amount: Amount::sats(200_000),
            })
            .then_expect_events(vec![expected_event]);
    }

    #[test]
    fn test_settle_lightning_invoice_underpaid() {
        let expected_event = mock_settled_event(Amount::sats(50_000), false, false);
        LightningInvoiceTestFramework::with(())
            .given(vec![mock_created_event()])
            .when(LightningInvoiceCommand::SettleInvoice {
                received_amount: Amount::sats(50_000),
            })
            .then_expect_events(vec![expected_event]);
    }
    #[test]
    fn test_create_lightning_invoice_already_exists() {
        let expected_error = Error::InvoiceAlreadyExists("123".to_string());
        LightningInvoiceTestFramework::with(())
            .given(vec![mock_created_event()])
            .when(LightningInvoiceCommand::CreateInvoice {
                invoice_id: "123".to_string(),
                node_id: "node1".to_string(),
                amount: Amount::sats(100_000),
                invoice: get_invoice(),
            })
            .then_expect_error(expected_error);
    }

    #[test]
    fn test_set_confirmed_lightning_invoice_already_confirmed() {
        LightningInvoiceTestFramework::with(())
            .given(vec![
                mock_created_event(),
                mock_settled_event(Amount::sats(100_000), false, true),
            ])
            .when(LightningInvoiceCommand::SettleInvoice {
                received_amount: Amount::sats(100_000),
            })
            .then_expect_events(vec![]);
    }

    fn mock_created_event() -> LightningInvoiceEvent {
        let invoice = get_invoice();
        LightningInvoiceEvent::InvoiceCreated {
            invoice_id: "123".to_string(),
            node_id: "node1".to_string(),
            amount: Amount::sats(100_000),
            invoice: invoice.to_string(),
            r_hash: invoice.payment_hash().to_string(),
        }
    }

    fn mock_settled_event(
        received_amount: Amount,
        overpaid: bool,
        paid: bool,
    ) -> LightningInvoiceEvent {
        LightningInvoiceEvent::InvoiceSettled {
            received_amount,
            overpaid,
            paid,
        }
    }

    fn get_invoice() -> Bolt11Invoice {
        Bolt11Invoice::from_str(
            "lntbs3m1pnf36h3pp5dm63f7meus5thxd3h23uqkfuydw340nrf6v8y398ga7tqjfrpnfsdq5w3jhxapqd9h8vmmfvdjscqzzsxq97ztucsp5yle6azm0tpy7h3dh0d6kmpzzzpyvzqkck476l96z5p5leqaraumq9qyyssqghpt4k54rrutwumlq6hav5wdjghlrxnyxe5dde37e5t4wwz4kkq3r5284l3rcnyzzqvry6xz4s8mq42npq8fzr7j9tvvuyh32xmh97gq0h8hdp"
        ).expect("valid invoice")
    }
}
