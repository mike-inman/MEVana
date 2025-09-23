#![allow(clippy::single_component_path_imports, dead_code, unused_variables)]

use bs58;
use normalizer_runtime::{
    layout_parser::{read_scalar, ScalarValue},
    layouts::{LayoutJson, LayoutRegistry},
    models::{LiquidityEvent, PoolSnapshot, SwapEvent},
};

// ---------- DLMM Pool → PoolSnapshot (minimal) ----------
pub fn decode_meteora_dlmm_pool_snapshot(
    reg: &LayoutRegistry,
    data: &[u8],
    pool_id_b58: &str,
    slot: u64,
) -> Option<PoolSnapshot> {
    let key = "meteora_dlmm/pool";
    let lj = reg.get(key)?;
    let LayoutJson::Fields { fields, .. } = lj else {
        return None;
    };

    let mut mint_x = [0u8; 32];
    let mut mint_y = [0u8; 32];
    let mut dec_x = 0u8;
    let mut dec_y = 0u8;
    let mut active = 0i32;
    let mut step = 0i32;
    let mut fee = 0u16;

    for f in fields {
        let v = read_scalar(data, f).ok()?;
        match (f.name.as_str(), v) {
            ("token_mint_x", ScalarValue::Pubkey(p)) => mint_x = p,
            ("token_mint_y", ScalarValue::Pubkey(p)) => mint_y = p,
            ("decimals_x", ScalarValue::U8(n)) => dec_x = n,
            ("decimals_y", ScalarValue::U8(n)) => dec_y = n,
            ("active_bin_id", ScalarValue::I32(n)) => active = n,
            ("bin_step", ScalarValue::I32(n)) => step = n,
            ("base_fee_bps", ScalarValue::U16(n)) => fee = n,
            _ => {}
        }
    }

    Some(PoolSnapshot {
        dex: "meteora-dlmm".into(),
        pool_id: pool_id_b58.into(),
        slot,
        token_a_mint: bs58::encode(mint_x).into_string(),
        token_b_mint: bs58::encode(mint_y).into_string(),
        decimals_a: dec_x,
        decimals_b: dec_y,
        // DLMM doesn't use these CLMM fields; leave neutral values.
        sqrt_price_x64: "0".into(),
        liquidity: "0".into(),
        // Reuse these integer slots to carry DLMM notions (handy in dashboards):
        tick_current: active, // active_bin_id
        tick_spacing: step,   // bin_step
        fee_bps_total: fee,
    })
}

// ---------- DAMM Pool → PoolSnapshot (for Meteora's constant product pools) ----------
pub fn decode_meteora_damm_pool_snapshot(
    data: &[u8],
    pool_id_b58: &str,
    slot: u64,
) -> Option<PoolSnapshot> {
    // Meteora DAMM pool layout (direct parsing, not IDL-based for now)
    // This needs the pool struct from meteora-damm IDL
    // For now, return None - needs proper implementation
    None
}

// ---------- DLMM Bin → minimal struct you’ll cache ----------
#[derive(Debug, Clone)]
pub struct DlmmBin {
    pub bin_id: i32,
    pub price: u128, // Q64.64 price
    pub liquidity_x: u128,
    pub liquidity_y: u128,
}

pub fn decode_meteora_bin(reg: &LayoutRegistry, data: &[u8]) -> Option<DlmmBin> {
    let key = "meteora_dlmm/bin";
    let lj = reg.get(key)?;
    let LayoutJson::Fields { fields, .. } = lj else {
        return None;
    };

    let mut bin_id = 0i32;
    let mut price = 0u128;
    let mut lx = 0u128;
    let mut ly = 0u128;
    for f in fields {
        let v = read_scalar(data, f).ok()?;
        match (f.name.as_str(), v) {
            ("bin_id", ScalarValue::I32(n)) => bin_id = n,
            ("price", ScalarValue::U128(n)) => price = n,
            ("liquidity_x", ScalarValue::U128(n)) => lx = n,
            ("liquidity_y", ScalarValue::U128(n)) => ly = n,
            _ => {}
        }
    }
    Some(DlmmBin {
        bin_id,
        price,
        liquidity_x: lx,
        liquidity_y: ly,
    })
}

// ---------- Event Extraction Functions ----------

/// Extract SwapEvent from Meteora DLMM swap instruction
pub fn extract_meteora_dlmm_swap_event(
    reg: &LayoutRegistry,
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<SwapEvent> {
    // Parse swap instruction data from DLMM IDL
    // This would need the swap instruction layout from the Meteora DLMM IDL
    // For now, return None - needs proper implementation
    None
}

/// Extract SwapEvent from Meteora DAMM swap instruction
pub fn extract_meteora_damm_swap_event(
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<SwapEvent> {
    // Parse swap instruction data from DAMM IDL
    // This would need the swap instruction layout from the Meteora DAMM IDL
    // For now, return None - needs proper implementation
    None
}

/// Extract LiquidityEvent from Meteora DLMM liquidity instructions
pub fn extract_meteora_dlmm_liquidity_event(
    reg: &LayoutRegistry,
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<LiquidityEvent> {
    // Parse liquidity instruction data (addLiquidity, removeLiquidity, etc.)
    // This would need the instruction layouts from the Meteora DLMM IDL
    // For now, return None - needs proper implementation
    None
}

// ---------- Legacy function for backward compatibility ----------
pub fn decode_meteora_pool_snapshot(
    reg: &LayoutRegistry,
    data: &[u8],
    pool_id_b58: &str,
    slot: u64,
) -> Option<PoolSnapshot> {
    decode_meteora_dlmm_pool_snapshot(reg, data, pool_id_b58, slot)
}
