use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum Currency {
    Btc,
    Usd,
    Eur,
    Aud,
    Gbp,
    Cad,
}

impl Display for Currency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::Btc => write!(f, "BTC"),
            Currency::Usd => write!(f, "USD"),
            Currency::Eur => write!(f, "EUR"),
            Currency::Cad => write!(f, "CAD"),
            Currency::Gbp => write!(f, "GBP"),
            Currency::Aud => write!(f, "AUD"),
        }
    }
}
