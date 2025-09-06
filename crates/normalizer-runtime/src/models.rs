// crates/normalizer-runtime/src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSnapshot {
    pub dex: String,
    pub pool_id: String,
    pub slot: u64,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub sqrt_price_x64: String,
    pub liquidity: String,
    pub tick_current: i32,
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
pub struct LiquidityEvent {
    pub dex: String,
    pub pool_id: String,
    pub slot: u64,
    pub sig: String,
    pub action: String,
    pub range_kind: String,
    pub range_low: i32,
    pub range_high: i32,
    pub amount_a: String,
    pub amount_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteUpdate {
    pub pair: String,
    pub dex: String,
    pub pool_id: String,
    pub slot: u64,
    pub test_size_mint: String,
    pub test_size_amount: String,
    pub effective_price_a_per_b: f64,
}
