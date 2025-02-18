use async_trait::async_trait;
use cqrs_es::{Aggregate, DomainEvent};
use payday_core::api::on_chain_api::OnChainTransactionEvent;
use payday_core::payment::amount::Amount;
use payday_core::payment::currency::Currency;
use payday_core::payment::invoice::{Error, InvoiceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcOnChainInvoice {
    pub invoice_id: InvoiceId,
    pub node_id: String,
    pub address: String,
    pub amount: Amount,
    pub received_amount: Amount,
    pub confirmations: u64,
    pub transaction_id: Option<String>,
    pub underpayment: bool,
    pub overpayment: bool,
    pub paid: bool,
}

impl Default for BtcOnChainInvoice {
    fn default() -> Self {
        Self {
            invoice_id: "".to_string(),
            node_id: "".to_string(),
            address: "".to_string(),
            amount: Amount::zero(Currency::Btc),
            received_amount: Amount::zero(Currency::Btc),
            confirmations: 0,
            transaction_id: None,
            underpayment: false,
            overpayment: false,
            paid: false,
        }
    }
}

#[async_trait]
pub trait OnChainInvoiceService: Send + Sync {}

pub struct OnChainInvoiceServices {}

#[derive(Debug, Deserialize)]
pub enum OnChainInvoiceCommand {
    CreateInvoice {
        invoice_id: InvoiceId,
        node_id: String,
        amount: Amount,
        address: String,
    },
    SetPending {
        amount: Amount,
    },
    SetConfirmed {
        confirmations: u64,
        amount: Amount,
        transaction_id: String,
    },
}

#[derive(Debug)]
pub struct OnChainCommand {
    pub id: String,
    pub command: OnChainInvoiceCommand,
}

impl From<OnChainTransactionEvent> for OnChainCommand {
    fn from(value: OnChainTransactionEvent) -> Self {
        let (aggregate_id, command) = match value {
            OnChainTransactionEvent::ReceivedConfirmed(tx) => (
                tx.address,
                OnChainInvoiceCommand::SetConfirmed {
                    confirmations: tx.confirmations as u64,
                    amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
                    transaction_id: tx.tx_id.to_owned(),
                },
            ),
            OnChainTransactionEvent::ReceivedUnconfirmed(tx) => (
                tx.address,
                OnChainInvoiceCommand::SetPending {
                    amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
                },
            ),
            OnChainTransactionEvent::SentConfirmed(tx) => (
                tx.address,
                OnChainInvoiceCommand::SetConfirmed {
                    confirmations: tx.confirmations as u64,
                    amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
                    transaction_id: tx.tx_id.to_owned(),
                },
            ),
            OnChainTransactionEvent::SentUnconfirmed(tx) => (
                tx.address,
                OnChainInvoiceCommand::SetPending {
                    amount: Amount::new(Currency::Btc, tx.amount.to_sat()),
                },
            ),
        };
        OnChainCommand {
            id: aggregate_id.to_string(),
            command,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OnChainInvoiceEvent {
    InvoiceCreated {
        invoice_id: InvoiceId,
        node_id: String,
        amount: Amount,
        address: String,
    },
    PaymentPending {
        received_amount: Amount,
        underpayment: bool,
        overpayment: bool,
    },
    PaymentConfirmed {
        received_amount: Amount,
        underpayment: bool,
        overpayment: bool,
        confirmations: u64,
        transaction_id: String,
    },
}

impl DomainEvent for OnChainInvoiceEvent {
    fn event_type(&self) -> String {
        let event_type = match self {
            OnChainInvoiceEvent::InvoiceCreated { .. } => "OnChainInvoiceCreated",
            OnChainInvoiceEvent::PaymentPending { .. } => "OnChainPaymentPending",
            OnChainInvoiceEvent::PaymentConfirmed { .. } => "OnChainPaymentConfirmed",
        };
        event_type.to_string()
    }

    fn event_version(&self) -> String {
        "1.0.0".to_string()
    }
}

#[async_trait]
impl Aggregate for BtcOnChainInvoice {
    type Command = OnChainInvoiceCommand;
    type Event = OnChainInvoiceEvent;
    type Error = Error;
    type Services = ();

    fn aggregate_type() -> String {
        "BtcOnChainInvoice".to_string()
    }

    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            OnChainInvoiceCommand::CreateInvoice {
                invoice_id,
                node_id,
                amount,
                address,
            } => {
                if amount.currency != Currency::Btc {
                    return Err(Error::InvalidCurrency(
                        amount.currency.to_string(),
                        Currency::Btc.to_string(),
                    ));
                }

                Ok(vec![OnChainInvoiceEvent::InvoiceCreated {
                    invoice_id,
                    node_id,
                    amount,
                    address: address.to_string(),
                }])
            }
            OnChainInvoiceCommand::SetPending { amount } => {
                Ok(vec![OnChainInvoiceEvent::PaymentPending {
                    received_amount: amount,
                    underpayment: amount.amount < self.amount.amount,
                    overpayment: amount.amount > self.amount.amount,
                }])
            }
            OnChainInvoiceCommand::SetConfirmed {
                confirmations,
                amount,
                transaction_id,
            } => Ok(vec![OnChainInvoiceEvent::PaymentConfirmed {
                received_amount: amount,
                underpayment: amount.amount < self.amount.amount,
                overpayment: amount.amount > self.amount.amount,
                confirmations,
                transaction_id,
            }]),
        }
    }

    fn apply(&mut self, event: Self::Event) {
        match event {
            OnChainInvoiceEvent::InvoiceCreated {
                invoice_id,
                node_id,
                amount,
                address,
            } => {
                self.invoice_id = invoice_id;
                self.node_id = node_id;
                self.amount = amount;
                self.address = address.to_string();
            }
            OnChainInvoiceEvent::PaymentPending {
                received_amount,
                underpayment,
                overpayment,
            } => {
                self.received_amount = received_amount;
                self.underpayment = underpayment;
                self.overpayment = overpayment;
            }
            OnChainInvoiceEvent::PaymentConfirmed {
                received_amount,
                underpayment,
                overpayment,
                confirmations,
                transaction_id,
            } => {
                self.received_amount = received_amount;
                self.underpayment = underpayment;
                self.overpayment = overpayment;
                self.confirmations = confirmations;
                self.paid = true;
                self.transaction_id = Some(transaction_id);
            }
        }
    }
}

#[cfg(test)]
mod aggregate_tests {
    use cqrs_es::test::TestFramework;
    use payday_core::payment::currency::Currency;

    use super::*;

    type OnChainInvoiceTestFramework = TestFramework<BtcOnChainInvoice>;

    #[test]
    fn test_create_invoice() {
        let expected = mock_created_event(100_000);
        OnChainInvoiceTestFramework::with(())
            .given_no_previous_events()
            .when(OnChainInvoiceCommand::CreateInvoice {
                invoice_id: "123".to_string(),
                node_id: "node1".to_string(),
                amount: amount_fn(100_000),
                address: "tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4".to_string(),
            })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_set_pending() {
        let amount = amount_fn(100_000);
        let expected = mock_pending_event(amount.amount, false, false);
        OnChainInvoiceTestFramework::with(())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetPending { amount })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_pending_overpayment() {
        let amount = amount_fn(100_001);
        let expected = mock_pending_event(amount.amount, false, true);
        OnChainInvoiceTestFramework::with(())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetPending { amount })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_pending_underpayment() {
        let amount = amount_fn(99_999);
        let expected = mock_pending_event(amount.amount, true, false);
        OnChainInvoiceTestFramework::with(())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetPending { amount })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_set_confirmed() {
        let expected = OnChainInvoiceEvent::PaymentConfirmed {
            received_amount: Amount::new(Currency::Btc, 100_000),
            underpayment: false,
            overpayment: false,
            confirmations: 1,
            transaction_id: "txid".to_string(),
        };
        OnChainInvoiceTestFramework::with(())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetConfirmed {
                confirmations: 1,
                amount: Amount::new(Currency::Btc, 100_000),
                transaction_id: "txid".to_string(),
            })
            .then_expect_events(vec![expected])
    }

    fn amount_fn(amount: u64) -> Amount {
        Amount::new(Currency::Btc, amount)
    }

    fn mock_pending_event(
        amount: u64,
        underpayment: bool,
        overpayment: bool,
    ) -> OnChainInvoiceEvent {
        OnChainInvoiceEvent::PaymentPending {
            received_amount: amount_fn(amount),
            underpayment,
            overpayment,
        }
    }

    fn mock_created_event(amount: u64) -> OnChainInvoiceEvent {
        OnChainInvoiceEvent::InvoiceCreated {
            invoice_id: "123".to_string(),
            node_id: "node1".to_string(),
            amount: amount_fn(amount),
            address: "tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4".to_string(),
        }
    }
}
