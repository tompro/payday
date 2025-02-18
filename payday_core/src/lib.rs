use std::pin::Pin;

use tokio_stream::Stream;

pub use error::Error;

pub mod api;
pub mod date;
pub mod error;
pub mod events;
pub mod payment;
pub mod persistence;

pub type Result<T> = std::result::Result<T, Error>;
pub type PaydayStream<T> = Pin<Box<dyn Stream<Item = T>>>;
