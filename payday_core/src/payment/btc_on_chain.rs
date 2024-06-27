use async_trait::async_trait;
use bitcoin::Address;
use cqrs_es::{Aggregate, DomainEvent};
use serde::{Deserialize, Serialize};

use crate::PaydayResult;
use crate::payment::amount::Amount;
use crate::payment::currency::Currency;
use crate::payment::invoice::{InvoiceError, InvoiceId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcOnChainInvoice {
    pub invoice_id: InvoiceId,
    pub address: String,
    pub amount: Amount,
    pub received_amount: Amount,
    pub confirmations: u64,
    pub underpayment: bool,
    pub overpayment: bool,
    pub paid: bool,
}

impl Default for BtcOnChainInvoice {
    fn default() -> Self {
        Self {
            invoice_id: "".to_string(),
            address: "".to_string(),
            amount: Amount::zero(Currency::BTC),
            received_amount: Amount::zero(Currency::BTC),
            confirmations: 0,
            underpayment: false,
            overpayment: false,
            paid: false,
        }
    }
}

#[async_trait]
pub trait OnChainInvoiceService: Send + Sync {
    async fn new_address(&self) -> PaydayResult<Address>;
}

pub struct OnChainInvoiceServices {
    pub address_service: Box<dyn OnChainInvoiceService>,
}

#[derive(Debug, Deserialize)]
pub enum OnChainInvoiceCommand {
    CreateInvoice {
        invoice_id: InvoiceId,
        amount: Amount,
    },
    SetPending {
        amount: Amount,
    },
    SetConfirmed {
        confirmations: u64,
        amount: Amount,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OnChainInvoiceEvent {
    InvoiceCreated {
        invoice_id: InvoiceId,
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
    type Error = InvoiceError;
    type Services = OnChainInvoiceServices;

    fn aggregate_type() -> String {
        "BtcOnChainInvoice".to_string()
    }

    async fn handle(
        &self,
        command: Self::Command,
        service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            OnChainInvoiceCommand::CreateInvoice { invoice_id, amount } => {
                if amount.currency != Currency::BTC {
                    return Err(InvoiceError::InvalidCurrency(
                        amount.currency.to_string(),
                        Currency::BTC.to_string(),
                    ));
                }

                let address = service.address_service.new_address().await.map_err(|_| {
                    InvoiceError::ServiceError("cold not create on chain address".to_string())
                })?;

                Ok(vec![OnChainInvoiceEvent::InvoiceCreated {
                    invoice_id,
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
            } => Ok(vec![OnChainInvoiceEvent::PaymentConfirmed {
                received_amount: amount,
                underpayment: amount.amount < self.amount.amount,
                overpayment: amount.amount > self.amount.amount,
                confirmations,
            }]),
        }
    }

    fn apply(&mut self, event: Self::Event) {
        match event {
            OnChainInvoiceEvent::InvoiceCreated {
                invoice_id,
                amount,
                address,
            } => {
                self.invoice_id = invoice_id;
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
            } => {
                self.received_amount = received_amount;
                self.underpayment = underpayment;
                self.overpayment = overpayment;
                self.confirmations = confirmations;
                self.paid = true;
            }
        }
    }
}

#[cfg(test)]
mod aggregate_tests {
    use std::str::FromStr;

    use bitcoin::Network;
    use cqrs_es::test::TestFramework;

    use crate::payment::currency::Currency;

    use super::*;

    type OnChainInvoiceTestFramework = TestFramework<BtcOnChainInvoice>;

    #[test]
    fn test_create_invoice() {
        let expected = mock_created_event(100_000);
        OnChainInvoiceTestFramework::with(mock_address_service())
            .given_no_previous_events()
            .when(OnChainInvoiceCommand::CreateInvoice {
                invoice_id: "123".to_string(),
                amount: amount_fn(100_000),
            })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_set_pending() {
        let amount = amount_fn(100_000);
        let expected = mock_pending_event(amount.amount, false, false);
        OnChainInvoiceTestFramework::with(mock_address_service())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetPending { amount })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_pending_overpayment() {
        let amount = amount_fn(100_001);
        let expected = mock_pending_event(amount.amount, false, true);
        OnChainInvoiceTestFramework::with(mock_address_service())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetPending { amount })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_pending_underpayment() {
        let amount = amount_fn(99_999);
        let expected = mock_pending_event(amount.amount, true, false);
        OnChainInvoiceTestFramework::with(mock_address_service())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetPending { amount })
            .then_expect_events(vec![expected])
    }

    #[test]
    fn test_set_confirmed() {
        let expected = OnChainInvoiceEvent::PaymentConfirmed {
            received_amount: Amount::new(Currency::BTC, 100_000),
            underpayment: false,
            overpayment: false,
            confirmations: 1,
        };
        OnChainInvoiceTestFramework::with(mock_address_service())
            .given(vec![mock_created_event(100_000)])
            .when(OnChainInvoiceCommand::SetConfirmed {
                confirmations: 1,
                amount: Amount::new(Currency::BTC, 100_000),
            })
            .then_expect_events(vec![expected])
    }

    fn amount_fn(amount: u64) -> Amount {
        Amount::new(Currency::BTC, amount)
    }

    fn mock_confirmed_event(amount: u64, confirmations: u64) -> OnChainInvoiceEvent {
        OnChainInvoiceEvent::PaymentConfirmed {
            received_amount: amount_fn(amount),
            underpayment: false,
            overpayment: false,
            confirmations,
        }
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
            amount: amount_fn(100_000),
            address: "tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4".to_string(),
        }
    }

    fn mock_address_service() -> OnChainInvoiceServices {
        OnChainInvoiceServices {
            address_service: Box::new(MockAddressService {}),
        }
    }

    struct MockAddressService;
    #[async_trait]
    impl OnChainInvoiceService for MockAddressService {
        async fn new_address(&self) -> PaydayResult<Address> {
            Ok(
                Address::from_str("tb1q6xm2qgh5r83lvmmu0v7c3d4wrd9k2uxu3sgcr4")
                    .unwrap()
                    .require_network(Network::Signet)
                    .unwrap(),
            )
        }
    }
}
