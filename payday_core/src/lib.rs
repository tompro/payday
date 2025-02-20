pub use error::Error;

pub mod api;
pub mod date;
pub mod error;
pub mod events;
pub mod payment;
pub mod persistence;

pub type Result<T> = std::result::Result<T, Error>;
