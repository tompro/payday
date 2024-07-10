use bitcoin::address::ParseError;
use bitcoin::amount::ParseAmountError;
use bitcoin::network::ParseNetworkError;

use crate::events::EventError;

#[derive(Debug)]
pub enum PaydayError {
    NodeConnectError(String),
    NodeApiError(String),
    DbError(String),
    InvalidBitcoinAddress(String),
    InvalidBitcoinNetwork(String),
    InvalidBitcoinAmount(String),
    EventError(String),
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

impl From<ParseAmountError> for PaydayError {
    fn from(value: ParseAmountError) -> Self {
        PaydayError::InvalidBitcoinAmount(value.to_string())
    }
}

impl From<EventError> for PaydayError {
    fn from(value: EventError) -> Self {
        match value {
            EventError::PublishError(m) => {
                PaydayError::EventError(format!("unable to publish event: {}", m))
            }
            EventError::SubscribeError(m) => {
                PaydayError::EventError(format!("unable to subscribe event stream: {}", m))
            }
        }
    }
}
