// crates/normalizer-runtime/src/lib.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod adapters;
pub mod layout_parser;
pub mod layouts;
pub mod models;
pub mod transaction;

/// ---------- Normalized universal swap shape ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedSwapV1 {
    // Routing / identity
    pub slot: u64,
    pub block_time_unix: i64,
    pub signature: String,
    pub ix_index: u16,
    pub program_id: String,
    pub dex: String,         // "raydium", "orca_whirlpool", "meteora", "jupiter"
    pub pool_id: String,     // main pool account (if known)
    pub swap_kind: SwapKind, // CPMM / CLMM / Aggregator / Other

    // Assets & amounts (raw chain units; decimals applied later by a mint cache)
    pub token_in_mint: String,
    pub token_out_mint: String,
    pub amount_in: u128,
    pub amount_out: u128,
    pub fee_bps_estimate: Option<u16>, // optional; we’ll estimate or inject when available

    // Price / market snapshot (raw ratio for now — we’ll add decimal-aware calc later)
    pub price: f64, // token_out per token_in

    // CLMM hints (if we can extract them)
    pub tick_index: Option<i32>,
    pub sqrt_price_x64: Option<u128>,

    // Provenance / sanity
    pub user_authority: Option<String>,
    pub success: bool,
    pub quality: QualityFlags,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SwapKind {
    Cpmm,
    Clmm,
    Aggregator,
    Other,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QualityFlags {
    pub idl_decoded: bool,
    pub from_profile: bool,
    pub amounts_from_meta: bool,
    pub pool_resolved: bool,
}

/// ---------- Transaction context we pass to extractors ----------
/// This is fed by your geyser/kafka consumer; keep it provider-agnostic.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenBalanceChange {
    pub owner: String, // base58
    pub mint: String,  // base58
    pub pre: u128,     // raw units
    pub post: u128,    // raw units
}

#[derive(Clone, Debug)]
pub struct TxContext<'a> {
    pub slot: u64,
    pub block_time_unix: i64,
    // pub signature: &'a str,
    pub signature: String,
    pub ix_index: u16,
    // pub program_id: &'a str,
    pub program_id: String,
    pub meta_ok: bool,

    /// All token balance changes in this transaction (pre/post merged into deltas)
    pub token_balance_changes: &'a [TokenBalanceChange],

    /// Optional: account data blob(s) if you cached them (e.g., CLMM state for the pool)
    /// map of base58 pubkey -> raw bytes
    pub account_data_by_pubkey: Option<&'a HashMap<String, Vec<u8>>>,

    /// Instruction/IDL decode artifacts (populated by codegen extractors in later steps)
    // pub decoded_name: Option<&'a str>,
    pub decoded_name: Option<String>,
    // pub decoded_args_json: Option<&'a serde_json::Value>,
    pub decoded_args_json: Option<serde_json::Value>,
    // pub decoded_accounts_by_role: Option<&'a HashMap<String, String>>, // role -> pubkey
    pub decoded_accounts_by_role: Option<std::collections::HashMap<String, String>>,
}

/// ---------- The trait our generated extractors will implement ----------
pub trait SwapExtractor: Send + Sync + 'static {
    fn program_id(&self) -> &str;

    /// Try to extract a normalized swap from this instruction context.
    /// Return None if the instruction is not a swap (or not supported).
    fn try_extract(&self, ctx: &TxContext) -> Option<NormalizedSwapV1>;
}

/// ---------- Helper: compute amounts & direction from meta ----------
/// Robust, DEX-agnostic: infer token_in/token_out and raw amounts from balance deltas.
/// If `owner_hint` is provided, we first look at deltas for that authority; otherwise we
/// aggregate across all owners seen in the tx (aggregators/CPIs may scatter accounts).
pub struct InferredAmounts {
    pub token_in_mint: String,
    pub token_out_mint: String,
    pub amount_in: u128,
    pub amount_out: u128,
}

pub fn infer_amounts_from_meta(
    changes: &[TokenBalanceChange],
    owner_hint: Option<&str>,
) -> Option<InferredAmounts> {
    // 1) Filter on owner if provided, else use all changes
    let iter = changes.iter().filter(|c| {
        if let Some(owner) = owner_hint {
            c.owner == owner
        } else {
            true
        }
    });

    // 2) Aggregate net delta per mint: post - pre
    let mut by_mint: HashMap<&str, i128> = HashMap::new();
    for c in iter {
        let pre = i128::try_from(c.pre).ok()?;
        let post = i128::try_from(c.post).ok()?;
        let delta = post - pre; // positive: gained; negative: spent
        *by_mint.entry(&c.mint).or_insert(0) += delta;
    }

    if by_mint.is_empty() {
        return None;
    }

    // 3) Find most negative (in) and most positive (out)
    let mut min_neg: Option<(&str, i128)> = None;
    let mut max_pos: Option<(&str, i128)> = None;

    for (mint, delta) in by_mint {
        if delta < 0 {
            if min_neg.is_none_or(|(_, v)| delta < v) {
                min_neg = Some((mint, delta));
            }
        } else if delta > 0 && max_pos.is_none_or(|(_, v)| delta > v) {
            max_pos = Some((mint, delta));
        }
    }

    let (in_mint, in_abs) = {
        let (m, d) = min_neg?;
        (m, (-d) as u128)
    };
    let (out_mint, out_abs) = {
        let (m, d) = max_pos?;
        (m, d as u128)
    };

    Some(InferredAmounts {
        token_in_mint: in_mint.to_string(),
        token_out_mint: out_mint.to_string(),
        amount_in: in_abs,
        amount_out: out_abs,
    })
}

/// Simple raw ratio; we’ll replace with decimal-aware pricing once we wire the mint-decimals cache.
pub fn compute_raw_price(amount_in: u128, amount_out: u128) -> f64 {
    if amount_in == 0 {
        return 0.0;
    }
    (amount_out as f64) / (amount_in as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_amounts_basic() {
        let changes = vec![
            TokenBalanceChange {
                owner: "U1".into(),
                mint: "SOL".into(),
                pre: 1_000_000,
                post: 900_000, // -100k in
            },
            TokenBalanceChange {
                owner: "U1".into(),
                mint: "USDC".into(),
                pre: 0,
                post: 2_000_000, // +2,000,000 out
            },
        ];

        let inferred = infer_amounts_from_meta(&changes, Some("U1")).unwrap();
        assert_eq!(inferred.token_in_mint, "SOL");
        assert_eq!(inferred.token_out_mint, "USDC");
        assert_eq!(inferred.amount_in, 100_000);
        assert_eq!(inferred.amount_out, 2_000_000);

        let price = compute_raw_price(inferred.amount_in, inferred.amount_out);
        assert!((price - 20.0).abs() < 1e-9);
    }
}
