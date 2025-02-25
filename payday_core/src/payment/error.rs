use std::fmt::{Display, Formatter};

use super::amount::Amount;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidAmount(Amount),
    InvalidCurrency(String, String),
    ServiceError(String),
    InvoiceAlreadyExists(String),
    InvoiceDetailsCreation(String),
    InvalidPaymentType(String),
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
            Error::InvoiceAlreadyExists(id) => write!(f, "Invoice already exists: {}", id),
            Error::InvoiceDetailsCreation(msg) => {
                write!(f, "Invoice details creation error: {}", msg)
            }
            Error::InvalidPaymentType(msg) => write!(f, "Invalid payment type: {}", msg),
        }
    }
}
