use bitcoin::address::ParseError;
use bitcoin::amount::ParseAmountError;
use bitcoin::network::ParseNetworkError;

#[derive(Debug)]
pub enum Error {
    NodeConnectError(String),
    NodeApiError(String),
    LightningPaymentFailed(String),
    InvalidInvoiceState(String),
    InvalidLightningInvoice(String),
    PublicKey(String),
    DbError(String),
    InvalidBitcoinAddress(String),
    InvalidBitcoinNetwork(String),
    InvalidBitcoinAmount(String),
    EventError(String),
    InvalidPaymentType(String),
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
