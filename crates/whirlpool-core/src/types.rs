use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

// On-chain packed Borsh Tick layout, verified against the IDL (2026-10-29).
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct Tick {
    pub initialized: bool,          // 1
    pub liquidity_net: i128,        // 16
    pub liquidity_gross: u128,      // 16
    pub fee_growth_outside_a: u128, // 16
    pub fee_growth_outside_b: u128, // 16
    pub reward_growths_outside: [u128; 3], // 48
                                    // Total per tick = 113 bytes
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct TickArray {
    pub start_tick_index: i32, // 4
    pub ticks: [Tick; 88],     // 9944
    pub whirlpool: Pubkey,     // 32; offset 9956 including discriminator
                               // Total account size including discriminator: 8 + 4 + 88*113 + 32 = 9988.
}

// Minimal subset of Whirlpool account (order matters — must match on-chain).
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct WhirlpoolMinimal {
    pub whirlpools_config: Pubkey,          // 32
    pub whirlpool_bump: [u8; 1],            // 1
    pub tick_spacing: u16,                  // 2
    pub tick_spacing_seed: [u8; 2],         // 2
    pub fee_rate: u16,                      // 2
    pub protocol_fee_rate: u16,             // 2
    pub liquidity: u128,                    // 16
    pub sqrt_price: u128,                   // 16 (Q64.64)
    pub tick_current_index: i32,            // 4
    pub protocol_fee_owed_a: u64,           // 8
    pub protocol_fee_owed_b: u64,           // 8
    pub token_mint_a: Pubkey,               // 32
    pub token_vault_a: Pubkey,              // 32
    pub fee_growth_global_a: u128,          // 16
    pub token_mint_b: Pubkey,               // 32
    pub token_vault_b: Pubkey,              // 32
    pub fee_growth_global_b: u128,          // 16
    pub reward_last_updated_timestamp: u64, // 8
    pub reward_infos: [RewardInfo; 3],      // 3 * 128
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct RewardInfo {
    pub mint: Pubkey,                   // 32
    pub vault: Pubkey,                  // 32
    pub authority: Pubkey,              // 32
    pub emissions_per_second_x64: u128, // 16
    pub growth_global_x64: u128,        // 16
}

#[inline]
pub fn tick_is_initialized(t: &Tick) -> bool {
    t.liquidity_gross > 0
}
