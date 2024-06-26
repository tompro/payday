use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::payment::currency::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    pub currency: Currency,
    pub amount: u64,
}

impl Amount {
    pub fn new(currency: Currency, amount: u64) -> Self {
        Self { currency, amount }
    }

    pub fn zero(currency: Currency) -> Self {
        Self {
            currency,
            amount: 0,
        }
    }
}

impl Default for Amount {
    fn default() -> Self {
        Self {
            currency: Currency::BTC,
            amount: 0,
        }
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}
