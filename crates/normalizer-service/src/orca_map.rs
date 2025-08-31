use bs58;
use normalizer_runtime::{
    layout_parser::{read_scalar, ScalarValue},
    layouts::{LayoutJson, LayoutRegistry},
    models::{LiquidityEvent, PoolSnapshot, SwapEvent},
};

use crate::kafka::{make_producer, publish_json_ff}; // you already have these

/// Decode Orca Whirlpool pool state into PoolSnapshot
pub fn decode_orca_whirlpool_snapshot(
    reg: &LayoutRegistry,
    data: &[u8],
    pool_id_b58: &str,
    slot: u64,
) -> Option<PoolSnapshot> {
    let key = "orca_whirlpool/pool";
    let lj = reg.get(key)?;
    let LayoutJson::Fields { fields, .. } = lj else {
        return None;
    };

    let mut token_mint_a = [0u8; 32];
    let mut token_mint_b = [0u8; 32];
    let mut token_vault_a = [0u8; 32];
    let mut token_vault_b = [0u8; 32];
    let mut fee_rate = 0u16;
    let mut protocol_fee_rate = 0u16;
    let mut liquidity = 0u128;
    let mut sqrt_price = 0u128;
    let mut tick_current_index = 0i32;
    let mut tick_spacing = 0i16;

    for f in fields {
        let v = read_scalar(data, f).ok()?;
        match (f.name.as_str(), v) {
            ("token_mint_a", ScalarValue::Pubkey(p)) => token_mint_a = p,
            ("token_mint_b", ScalarValue::Pubkey(p)) => token_mint_b = p,
            ("token_vault_a", ScalarValue::Pubkey(p)) => token_vault_a = p,
            ("token_vault_b", ScalarValue::Pubkey(p)) => token_vault_b = p,
            ("fee_rate", ScalarValue::U16(n)) => fee_rate = n,
            ("protocol_fee_rate", ScalarValue::U16(n)) => protocol_fee_rate = n,
            ("liquidity", ScalarValue::U128(n)) => liquidity = n,
            ("sqrt_price", ScalarValue::U128(n)) => sqrt_price = n,
            ("tick_current_index", ScalarValue::I32(n)) => tick_current_index = n,
            ("tick_spacing", ScalarValue::U16(n)) => tick_spacing = n as i16,
            _ => {}
        }
    }

    // Note: Orca doesn't store decimals in pool state, we'd need to fetch mint info
    // For now, we'll use placeholder decimals
    Some(PoolSnapshot {
        dex: "orca-whirlpool".into(),
        pool_id: pool_id_b58.into(),
        slot,
        token_a_mint: bs58::encode(token_mint_a).into_string(),
        token_b_mint: bs58::encode(token_mint_b).into_string(),
        decimals_a: 9, // placeholder - should be fetched from mint
        decimals_b: 9, // placeholder - should be fetched from mint
        sqrt_price_x64: sqrt_price.to_string(),
        liquidity: liquidity.to_string(),
        tick_current: tick_current_index,
        tick_spacing: tick_spacing as i32,
        fee_bps_total: fee_rate,
    })
}

#[derive(serde::Serialize)]
struct WhirlpoolVaultsMsg {
    pool_id: String,
    token_vault_a: String,
    token_vault_b: String,
    token_program: String,
}

/// Extract SwapEvent from Orca Whirlpool swap instruction
pub fn extract_orca_whirlpool_swap_event(
    reg: &LayoutRegistry,
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<SwapEvent> {
    // Parse swap_v2 instruction data from Orca Whirlpool IDL
    // This would need the swap_v2 instruction layout from the Orca Whirlpool IDL
    // For now, return None - needs proper implementation
    None
}

/// Extract LiquidityEvent from Orca Whirlpool liquidity instructions
pub fn extract_orca_whirlpool_liquidity_event(
    reg: &LayoutRegistry,
    instruction_data: &[u8],
    account_keys: &[String],
    tx_meta: &normalizer_runtime::TxContext,
) -> Option<LiquidityEvent> {
    // Parse liquidity instruction data (openPosition, increaseLiquidity, etc.)
    // This would need the instruction layouts from the Orca Whirlpool IDL
    // For now, return None - needs proper implementation
    None
}

/// Decode Orca Whirlpool tick array for liquidity analysis
#[derive(Debug, Clone)]
pub struct WhirlpoolTickArray {
    pub start_tick_index: i32,
    pub ticks: Vec<WhirlpoolTick>,
}

#[derive(Debug, Clone)]
pub struct WhirlpoolTick {
    pub tick_index: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_a: u128,
    pub fee_growth_outside_b: u128,
}

pub fn decode_orca_whirlpool_tick_array(
    reg: &LayoutRegistry,
    data: &[u8],
) -> Option<WhirlpoolTickArray> {
    let key = "orca_whirlpool/tickarray";
    let lj = reg.get(key)?;
    let LayoutJson::Fields { fields, .. } = lj else {
        return None;
    };

    let mut start_tick_index = 0i32;
    let mut ticks = Vec::new();

    for f in fields {
        let v = read_scalar(data, f).ok()?;
        match (f.name.as_str(), v) {
            ("start_tick_index", ScalarValue::I32(n)) => start_tick_index = n,
            // The ticks array would need to be parsed separately
            // This is a simplified version - full implementation would need array parsing
            _ => {}
        }
    }

    Some(WhirlpoolTickArray {
        start_tick_index,
        ticks, // Would be populated from array parsing
    })
}
