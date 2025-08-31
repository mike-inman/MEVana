use serde::{Deserialize, Serialize};

// Re-export your shared types for consistency
pub use normalizer_runtime as rt;
pub use rt::models as rt_models;

// Canonical pair key (base58 strings)
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Pair {
    pub a: String,
    pub b: String,
}
impl Pair {
    pub fn canonical(a: &str, b: &str) -> Self {
        if a <= b {
            Self {
                a: a.into(),
                b: b.into(),
            }
        } else {
            Self {
                a: b.into(),
                b: a.into(),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub dex: String,
    pub pool_id: String,
    pub pair: Pair,
    pub side_ab: bool,    // true = A->B, false = B->A
    pub amount_in: u128,  // smallest units
    pub amount_out: u128, // smallest units
    pub slot: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbSignal {
    pub pair: Pair,
    pub dex_sell: String,
    pub dex_buy: String,
    pub amount_in: u128,
    pub expected_profit_out: i128, // signed
    pub slot: u64,
}
