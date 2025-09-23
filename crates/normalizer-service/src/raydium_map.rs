#![allow(
    clippy::single_component_path_imports,
    dead_code,
    unused_variables,
    unused_imports
)]

use bs58;
use normalizer_runtime::{
    layout_parser::{parse_array_entries, read_scalar, ScalarValue},
    layouts::{LayoutJson, LayoutRegistry},
    models::{LiquidityEvent, PoolSnapshot, SwapEvent},
};

#[derive(Default, Debug)]
pub struct TickWindow {
    pub rows: Vec<(i32, u128, i128)>, // (tick_index, liq_gross, liq_net)
}

/// Decode Raydium CLMM tick array for liquidity analysis
pub fn decode_raydium_tick_array(reg: &LayoutRegistry, data: &[u8]) -> Option<TickWindow> {
    let key = "raydium_clmm/tickarray";
    let lj = reg.get(key)?;
    let LayoutJson::Array { array, .. } = lj else {
        return None;
    };
    let entries = parse_array_entries(data, array).ok()?;

    let mut out = TickWindow::default();
    for row in entries {
        let mut idx = 0i32;
        let mut lg = 0u128;
        let mut ln = 0i128;
        for (name, v) in row {
            match (name.as_str(), v) {
                ("tick_index", ScalarValue::I32(n)) => idx = n,
                ("liquidity_gross", ScalarValue::U128(n)) => lg = n,
                ("liquidity_net", ScalarValue::I32(n)) => ln = n as i128,
                ("liquidity_net", ScalarValue::I128(n)) => ln = n,
                _ => {}
            }
        }
        out.rows.push((idx, lg, ln));
    }
    Some(out)
}

/// Decode Raydium CLMM pool state into PoolSnapshot
pub fn decode_raydium_clmm_pool_snapshot(
    reg: &LayoutRegistry,
    data: &[u8],
    pool_id_b58: &str,
    slot: u64,
) -> Option<PoolSnapshot> {
    let key = "raydium_clmm/pool";
    let lj = reg.get(key)?;
    let LayoutJson::Fields { fields, .. } = lj else {
        return None;
    };

    let mut token_a = [0u8; 32];
    let mut token_b = [0u8; 32];
    let mut dec_a = 0u8;
    let mut dec_b = 0u8;
    let mut sqrt_price = 0u128;
    let mut liq = 0u128;
    let mut tick_cur = 0i32;
    let mut tick_spacing = 0i32;
    let mut fee_bps = 0u16;

    for f in fields {
        let v = read_scalar(data, f).ok()?;
        match (f.name.as_str(), v) {
            ("token_mint_0", ScalarValue::Pubkey(p)) => token_a = p,
            ("token_mint_1", ScalarValue::Pubkey(p)) => token_b = p,
            ("decimals_0", ScalarValue::U8(n)) => dec_a = n,
            ("decimals_1", ScalarValue::U8(n)) => dec_b = n,
            ("sqrt_price_x64", ScalarValue::U128(n)) => sqrt_price = n,
            ("liquidity", ScalarValue::U128(n)) => liq = n,
            ("tick_current", ScalarValue::I32(n)) => tick_cur = n,
            ("tick_spacing", ScalarValue::I32(n)) => tick_spacing = n,
            ("fee_bps", ScalarValue::U16(n)) => fee_bps = n,
            _ => {}
        }
    }

    Some(PoolSnapshot {
        dex: "raydium-clmm".into(),
        pool_id: pool_id_b58.into(),
        slot,
        token_a_mint: bs58::encode(token_a).into_string(),
        token_b_mint: bs58::encode(token_b).into_string(),
        decimals_a: dec_a,
        decimals_b: dec_b,
        sqrt_price_x64: sqrt_price.to_string(),
        liquidity: liq.to_string(),
        tick_current: tick_cur,
        tick_spacing,
        fee_bps_total: fee_bps,
    })
}

/// Decode Raydium AMM pool state (different from CLMM)
pub fn decode_raydium_amm_pool_snapshot(
    data: &[u8],
    pool_id_b58: &str,
    slot: u64,
) -> Option<PoolSnapshot> {
    // Raydium AMM pool layout (direct parsing, not IDL-based for now)
    // This needs the AmmInfo struct from raydium-amm IDL
    // For now, return None - needs proper implementation
    None
}

/// Extract SwapEvent from Raydium CLMM swap instruction
pub fn extract_raydium_clmm_swap_event(
    reg: &LayoutRegistry,
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<SwapEvent> {
    // Parse swap instruction data
    // This would need the swap instruction layout from the Raydium CLMM IDL
    // For now, return None - needs proper implementation
    None
}

/// Extract SwapEvent from Raydium AMM swap instruction
pub fn extract_raydium_amm_swap_event(
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<SwapEvent> {
    // Parse swap instruction data
    // This would need the swap instruction layout from the Raydium AMM IDL
    // For now, return None - needs proper implementation
    None
}

/// Extract LiquidityEvent from Raydium CLMM liquidity instructions
pub fn extract_raydium_clmm_liquidity_event(
    reg: &LayoutRegistry,
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<LiquidityEvent> {
    // Parse liquidity instruction data (openPosition, closePosition, etc.)
    // This would need the instruction layouts from the Raydium CLMM IDL
    // For now, return None - needs proper implementation
    None
}
