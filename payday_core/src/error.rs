use bitcoin::address::ParseError;
use bitcoin::network::ParseNetworkError;

pub type PaydayResult<T> = Result<T, PaydayError>;

#[derive(Debug)]
pub enum PaydayError {
    NodeConnectError(String),
    NodeApiError(String),
    InvalidBitcoinAddress(String),
    InvalidBitcoinNetwork(String),
}

impl From<ParseNetworkError> for PaydayError {
    fn from(value: ParseNetworkError) -> Self {
        PaydayError::InvalidBitcoinNetwork(value.to_string())
    }
}
impl From<ParseError> for PaydayError {
    fn from(value: ParseError) -> Self {
        PaydayError::InvalidBitcoinAddress(value.to_string())
    }
}
