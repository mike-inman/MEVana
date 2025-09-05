use serde::{Deserialize, Serialize};
pub use shared_protocol::{ArbSignalV1 as ArbSignal, Pair};

// Re-export your shared types for consistency
pub use normalizer_runtime as rt;
pub use rt::models as rt_models;

// Pair & ArbSignal now sourced from shared-protocol crate

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

// ArbSignal struct migrated to shared-protocol (ArbSignalV1)
