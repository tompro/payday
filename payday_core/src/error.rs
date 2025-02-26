use bitcoin::address::ParseError;
use bitcoin::amount::ParseAmountError;
use bitcoin::network::ParseNetworkError;
use cqrs_es::AggregateError;

use crate::payment;

#[derive(Debug)]
pub enum Error {
    NodeConnect(String),
    NodeApi(String),
    LightningPaymentFailed(String),
    InvalidInvoiceState(String),
    InvalidLightningInvoice(String),
    PublicKey(String),
    Db(String),
    InvalidBitcoinAddress(String),
    InvalidBitcoinNetwork(String),
    InvalidBitcoinAmount(String),
    Event(String),
    InvalidPaymentType(String),
    Payment(String),
    PaymentProcessing(String),
}

impl From<ParseNetworkError> for Error {
    fn from(value: ParseNetworkError) -> Self {
        Error::InvalidBitcoinNetwork(value.to_string())
    }
}
impl From<ParseError> for Error {
    fn from(value: ParseError) -> Self {
        Error::InvalidBitcoinAddress(value.to_string())
    }
}

impl From<ParseAmountError> for Error {
    fn from(value: ParseAmountError) -> Self {
        Error::InvalidBitcoinAmount(value.to_string())
    }
}

impl From<bitcoin::key::ParsePublicKeyError> for Error {
    fn from(value: bitcoin::key::ParsePublicKeyError) -> Self {
        Error::PublicKey(value.to_string())
    }
}

impl From<lightning_invoice::ParseOrSemanticError> for Error {
    fn from(value: lightning_invoice::ParseOrSemanticError) -> Self {
        Error::InvalidLightningInvoice(value.to_string())
    }
}

impl From<payment::Error> for Error {
    fn from(value: payment::Error) -> Self {
        Error::Payment(value.to_string())
    }
}

impl From<AggregateError<payment::Error>> for Error {
    fn from(value: AggregateError<payment::Error>) -> Self {
        match value {
            AggregateError::UserError(e) => Error::Payment(e.to_string()),
            _ => Error::PaymentProcessing(value.to_string()),
        }
    }
}
