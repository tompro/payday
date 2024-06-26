use std::pin::Pin;

use tokio_stream::Stream;

pub use error::PaydayError;

pub mod error;
pub mod node;
mod payment;

pub type PaydayResult<T> = Result<T, PaydayError>;
pub type PaydayStream<T> = Pin<Box<dyn Stream<Item = T>>>;
