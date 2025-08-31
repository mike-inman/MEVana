use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Mint(pub String); // base58

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Pair {
    pub a: Mint,
    pub b: Mint,
}
impl Pair {
    pub fn canonical(a: &str, b: &str) -> Self {
        if a <= b {
            Self {
                a: Mint(a.into()),
                b: Mint(b.into()),
            }
        } else {
            Self {
                a: Mint(b.into()),
                b: Mint(a.into()),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSnapshot {
    pub dex: String,
    pub pool_id: String,
    pub slot: u64,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub sqrt_price_x64: String, // as decimal string
    pub liquidity: String,      // as decimal string
    pub tick_current: i32,      // signed (Whirlpool/Raydium CLMM)
    pub tick_spacing: i32,
    pub fee_bps_total: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub dex: String,
    pub pool_id: String,
    pub slot: u64,
    pub sig: String,
    pub in_mint: String,
    pub out_mint: String,
    pub amount_in: String,
    pub amount_out: String,
    pub fee_paid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DexEvent {
    PoolUpdate(PoolSnapshot),
    Swap(SwapEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub dex: String,
    pub pool_id: String,
    pub pair: Pair,       // canonical
    pub side_ab: bool,    // true = A->B, false = B->A
    pub amount_in: u128,  // in smallest units of input token
    pub amount_out: u128, // in smallest units of output token
    pub slot: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbSignal {
    pub pair: Pair,
    pub dex_sell: String,
    pub dex_buy: String,
    pub amount_in: u128,
    pub expected_profit_out: i128, // signed; smallest units of base token (pair.a or pair.b depending on cycle)
    pub slot: u64,
}
