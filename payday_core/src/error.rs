use bitcoin::address::ParseError;
use bitcoin::amount::ParseAmountError;
use bitcoin::network::ParseNetworkError;

use crate::events::MessageError;

#[derive(Debug)]
pub enum Error {
    NodeConnectError(String),
    NodeApiError(String),
    LightningPaymentFailed(String),
    InvalidInvoiceState(String),
    PublicKey(String),
    DbError(String),
    InvalidBitcoinAddress(String),
    InvalidBitcoinNetwork(String),
    InvalidBitcoinAmount(String),
    EventError(String),
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

impl From<MessageError> for Error {
    fn from(value: MessageError) -> Self {
        match value {
            MessageError::PublishError(m) => {
                Error::EventError(format!("unable to publish event: {}", m))
            }
            MessageError::SubscribeError(m) => {
                Error::EventError(format!("unable to subscribe event stream: {}", m))
            }
            MessageError::ConfirmError(m) => {
                Error::EventError(format!("unable to confirm event processing: {}", m))
            }
        }
    }
}
