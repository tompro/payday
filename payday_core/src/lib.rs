use std::pin::Pin;

use tokio_stream::Stream;

pub use error::PaydayError;

pub mod date;
pub mod error;
pub mod events;
pub mod payment;
pub mod persistence;

pub type PaydayResult<T> = Result<T, PaydayError>;
pub type PaydayStream<T> = Pin<Box<dyn Stream<Item = T>>>;
