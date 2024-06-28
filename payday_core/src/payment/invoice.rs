use std::fmt::{Display, Formatter};

use async_trait::async_trait;
use cqrs_es::{Aggregate, DomainEvent};
use serde::{Deserialize, Serialize};

use crate::payment::amount::Amount;

pub type InvoiceId = String;
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

trait CreateInvoiceService {
    fn create_invoice(&self, invoice: InvoiceData) -> InvoiceResult<InvoiceEvent>;
}

#[derive(Debug, Deserialize)]
pub struct InvoiceData {
    invoice_id: InvoiceId,
    amount: Amount,
}

#[derive(Debug, Deserialize)]
pub enum InvoiceCommand {
    CreateInvoice {
        invoice_id: InvoiceId,
        amount: Amount,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvoiceEvent {
    InvoiceCreated {
        invoice_id: InvoiceId,
        amount: Amount,
    },
}

impl DomainEvent for InvoiceEvent {
    fn event_type(&self) -> String {
        let event_type = match self {
            InvoiceEvent::InvoiceCreated { .. } => "InvoiceCreated",
        };
        event_type.to_string()
    }

    fn event_version(&self) -> String {
        "1.0.0".to_string()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: InvoiceId,
    pub amount: Amount,
}

#[async_trait]
impl Aggregate for Invoice {
    type Command = InvoiceCommand;
    type Event = InvoiceEvent;
    type Error = InvoiceError;
    type Services = ();

    fn aggregate_type() -> String {
        "Invoice".to_string()
    }

    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            InvoiceCommand::CreateInvoice { invoice_id, amount } => {
                Ok(vec![InvoiceEvent::InvoiceCreated { invoice_id, amount }])
            }
        }
    }

    fn apply(&mut self, event: Self::Event) {
        match event {
            InvoiceEvent::InvoiceCreated { invoice_id, amount } => {
                self.invoice_id = invoice_id;
                self.amount = amount;
            }
        }
    }
}

#[cfg(test)]
mod aggregate_tests {
    use cqrs_es::test::TestFramework;

    use crate::payment::currency::Currency;

    use super::*;

    type InvoiceTestFramework = TestFramework<Invoice>;

    #[test]
    fn test_invoice() {
        let expected = InvoiceEvent::InvoiceCreated {
            invoice_id: "123".to_string(),
            amount: Amount::new(Currency::Btc, 100_000),
        };

        InvoiceTestFramework::with(())
            .given_no_previous_events()
            .when(InvoiceCommand::CreateInvoice {
                invoice_id: "123".to_string(),
                amount: Amount::new(Currency::Btc, 100_000),
            })
            .then_expect_events(vec![expected])
    }
}
