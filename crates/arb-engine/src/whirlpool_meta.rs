use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use normalizer_runtime::models::PoolSnapshot;
use whirlpool_core::pda::{derive_tick_array_address, three_tick_arrays_around};

// What we need at execution time
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhirlpoolMeta {
    pub whirlpool: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub token_program: Pubkey, // Tokenkeg… (or Token-2022 if you choose)
    pub tick_current: i32,
    pub tick_spacing: i32,
}

// Global, lock-free map keyed by base58 pool id
pub static WHIRL_META: Lazy<DashMap<String, WhirlpoolMeta>> = Lazy::new(|| DashMap::new());

#[inline]
pub fn get_meta(pool_id: &str) -> Option<WhirlpoolMeta> {
    WHIRL_META.get(pool_id).map(|v| v.clone())
}

// Update only tick fields from PoolSnapshot
pub fn upsert_from_snapshot(s: &PoolSnapshot) {
    use std::str::FromStr;
    let pid = s.pool_id.clone();
    if let Some(mut ex) = WHIRL_META.get_mut(&pid) {
        ex.tick_current = s.tick_current;
        ex.tick_spacing = s.tick_spacing;
    } else {
        // We may not know vaults yet; leave them as default until meta arrives
        let whirlpool = Pubkey::from_str(&s.pool_id).unwrap_or(Pubkey::default());
        let tma = Pubkey::from_str(&s.token_a_mint).unwrap_or(Pubkey::default());
        let tmb = Pubkey::from_str(&s.token_b_mint).unwrap_or(Pubkey::default());
        WHIRL_META.insert(
            pid,
            WhirlpoolMeta {
                whirlpool,
                token_mint_a: tma,
                token_mint_b: tmb,
                token_vault_a: Pubkey::default(),
                token_vault_b: Pubkey::default(),
                token_program: spl_token::ID, // default classic Tokenkeg…; override via meta
                tick_current: s.tick_current,
                tick_spacing: s.tick_spacing,
            },
        );
    }
}

// Message shape published by normalizer-service for vaults/etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhirlpoolVaultsMsg {
    pub pool_id: String,
    pub token_vault_a: String,
    pub token_vault_b: String,
    pub token_program: String, // base58 program id
}

pub fn upsert_vaults(msg: &WhirlpoolVaultsMsg) {
    use std::str::FromStr;
    let pid = &msg.pool_id;
    if let Some(mut ex) = WHIRL_META.get_mut(pid) {
        ex.token_vault_a = Pubkey::from_str(&msg.token_vault_a).unwrap_or(Pubkey::default());
        ex.token_vault_b = Pubkey::from_str(&msg.token_vault_b).unwrap_or(Pubkey::default());
        ex.token_program = Pubkey::from_str(&msg.token_program).unwrap_or(spl_token::ID);
    } else {
        // if snapshot hasn't come yet, initialize a partial entry
        WHIRL_META.insert(
            pid.clone(),
            WhirlpoolMeta {
                whirlpool: Pubkey::from_str(pid).unwrap_or(Pubkey::default()),
                token_mint_a: Pubkey::default(),
                token_mint_b: Pubkey::default(),
                token_vault_a: Pubkey::from_str(&msg.token_vault_a).unwrap_or(Pubkey::default()),
                token_vault_b: Pubkey::from_str(&msg.token_vault_b).unwrap_or(Pubkey::default()),
                token_program: Pubkey::from_str(&msg.token_program).unwrap_or(spl_token::ID),
                tick_current: 0,
                tick_spacing: 1,
            },
        );
    }
}

// Whirlpool oracle PDA (v2): seeds ["oracle", whirlpool]
#[inline]
pub fn oracle_pda(program_id: &Pubkey, whirlpool: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"oracle", whirlpool.as_ref()], program_id).0
}

// Three tick array PDAs around current tick
#[inline]
pub fn three_tick_arrays(program_id: &Pubkey, meta: &WhirlpoolMeta) -> [Pubkey; 3] {
    let (t0, t1, t2) = three_tick_arrays_around(meta.tick_current, meta.tick_spacing);
    let (a0, _) = derive_tick_array_address(program_id, &meta.whirlpool, t0);
    let (a1, _) = derive_tick_array_address(program_id, &meta.whirlpool, t1);
    let (a2, _) = derive_tick_array_address(program_id, &meta.whirlpool, t2);
    [a0, a1, a2]
}
