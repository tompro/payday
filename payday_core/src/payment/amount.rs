use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use super::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    pub currency: Currency,
    pub cent_amount: u64,
}

impl Amount {
    pub fn new(currency: Currency, cent_amount: u64) -> Self {
        Self {
            currency,
            cent_amount,
        }
    }

    pub fn zero(currency: Currency) -> Self {
        Self {
            currency,
            cent_amount: 0,
        }
    }

    pub fn sats(sats: u64) -> Self {
        Self {
            currency: Currency::Btc,
            cent_amount: sats,
        }
    }
}

impl Default for Amount {
    fn default() -> Self {
        Self {
            currency: Currency::Btc,
            cent_amount: 0,
        }
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.cent_amount, self.currency)
    }
}
