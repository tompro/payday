pub use error::Error;

pub mod aggregate;
pub mod api;
pub mod date;
pub mod error;
pub mod payment;
pub mod persistence;
pub mod processor;

pub type Result<T> = std::result::Result<T, Error>;
