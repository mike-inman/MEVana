// crates/shared-core/src/lib.rs
//! Shared utilities and types for MEVana-MCP components

use serde::{Deserialize, Serialize};
use std::fmt;

/// Common error type for shared components
pub type Result<T> = std::result::Result<T, Error>;

/// Custom error type
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::new(&err.to_string())
    }
}

/// Common configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub endpoint: String,
    pub x_token: String,
    pub program_ids: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: "https://grpc.ny.shyft.to".to_string(),
            x_token: "".to_string(),
            program_ids: vec![],
        }
    }
}

/// Transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub signature: String,
    pub slot: u64,
    pub program_ids: Vec<String>,
    pub accounts: Vec<String>,
    pub timestamp: u64,
}

/// Pool state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub address: String,
    pub dex: String,
    pub token_a: String,
    pub token_b: String,
    pub liquidity: String,
    pub price: Option<f64>,
    pub slot: u64,
}

/// DEX program identifiers
pub mod programs {
    pub const RAYDIUM_AMM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
    pub const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqGnT";
    pub const METEORA_DLMM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
    pub const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
    pub const JUPITER_V6: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";
    pub const OPENBOOK: &str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";
    pub const PUMP_FUN: &str = "6EF8rrecthR5Dkzon8NQtPiZzmEg7WRKNgDh5kJcZ5M";

    /// Get all known DEX program IDs
    pub fn all_dex_programs() -> Vec<&'static str> {
        vec![
            RAYDIUM_AMM,
            RAYDIUM_CLMM,
            METEORA_DLMM,
            ORCA_WHIRLPOOL,
            JUPITER_V6,
            OPENBOOK,
            PUMP_FUN,
        ]
    }

    /// Get program name from ID
    pub fn program_name(program_id: &str) -> &'static str {
        match program_id {
            RAYDIUM_AMM => "Raydium AMM",
            RAYDIUM_CLMM => "Raydium CLMM",
            METEORA_DLMM => "Meteora DLMM",
            ORCA_WHIRLPOOL => "Orca Whirlpool",
            JUPITER_V6 => "Jupiter V6",
            OPENBOOK => "OpenBook",
            PUMP_FUN => "Pump.fun",
            _ => "Unknown DEX",
        }
    }
}

/// Utility functions
pub mod utils {
    use super::*;

    /// Convert lamports to SOL
    pub fn lamports_to_sol(lamports: u64) -> f64 {
        lamports as f64 / 1_000_000_000.0
    }

    /// Convert SOL to lamports
    pub fn sol_to_lamports(sol: f64) -> u64 {
        (sol * 1_000_000_000.0) as u64
    }

    /// Calculate percentage difference between two values
    pub fn percentage_diff(old_val: f64, new_val: f64) -> f64 {
        if old_val == 0.0 {
            return 0.0;
        }
        ((new_val - old_val) / old_val) * 100.0
    }

    /// Format number as basis points
    pub fn to_bps(value: f64) -> i64 {
        (value * 100.0) as i64
    }
}
