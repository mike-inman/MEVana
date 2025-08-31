use arc_swap::ArcSwap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Knobs {
    pub min_profit_lamports: i64,        // e.g., 5000
    pub max_slippage_bps: u16,           // e.g., 30
    pub priority_fee_microlamports: u64, // per CU
    pub test_amount_in_base: u128,       // fixed probe size
    pub enable_flash_loans: bool,
    pub max_hops: u8, // 2 or 3
}

impl Default for Knobs {
    fn default() -> Self {
        Self {
            min_profit_lamports: 5_000,
            max_slippage_bps: 30,
            priority_fee_microlamports: 2_000, // tune on chain congestion
            test_amount_in_base: 100_000,      // adjust per pair/decimals
            enable_flash_loans: true,
            max_hops: 2,
        }
    }
}

pub static KNOBS: once_cell::sync::Lazy<ArcSwap<Knobs>> =
    once_cell::sync::Lazy::new(|| ArcSwap::from_pointee(Knobs::default()));

pub fn get_knobs() -> Arc<Knobs> {
    KNOBS.load_full()
}
pub fn set_knobs(newk: Knobs) {
    KNOBS.store(Arc::new(newk));
}
