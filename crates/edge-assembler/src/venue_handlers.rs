#![allow(dead_code)]

use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum VenueId {
    OrcaWhirlpool = 1,
    RaydiumCpmm = 2,
    MeteoraDlmm = 3,
    Phoenix = 4,
    RaydiumClmm = 5,
}

impl VenueId {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(VenueId::OrcaWhirlpool),
            2 => Some(VenueId::RaydiumCpmm),
            3 => Some(VenueId::MeteoraDlmm),
            4 => Some(VenueId::Phoenix),
            5 => Some(VenueId::RaydiumClmm),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            VenueId::OrcaWhirlpool => "OrcaWhirlpool",
            VenueId::RaydiumCpmm => "RaydiumCpmm",
            VenueId::MeteoraDlmm => "MeteoraDlmm",
            VenueId::Phoenix => "Phoenix",
            VenueId::RaydiumClmm => "RaydiumClmm",
        }
    }

    pub fn program_id(&self) -> &'static str {
        match self {
            VenueId::OrcaWhirlpool => "whirLbMi4jYofpSBpZUVG3WbNDLj1L9Vt1k98ajJRJm",
            VenueId::RaydiumCpmm => "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            VenueId::MeteoraDlmm => "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
            VenueId::Phoenix => "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY",
            VenueId::RaydiumClmm => "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
        }
    }
}

// Venue-specific update handlers
pub trait VenueHandler {
    fn handle_account_update(
        &self,
        account_data: &[u8],
        slot: u64,
        write_version: u64,
    ) -> anyhow::Result<Vec<crate::edge::EdgeDelta>>;
    fn venue_id(&self) -> VenueId;
}

#[derive(Clone)]
pub struct OrcaWhirlpoolHandler;

impl VenueHandler for OrcaWhirlpoolHandler {
    fn handle_account_update(
        &self,
        account_data: &[u8],
        slot: u64,
        write_version: u64,
    ) -> anyhow::Result<Vec<crate::edge::EdgeDelta>> {
        use borsh::BorshDeserialize;
        use whirlpool_core::types::WhirlpoolMinimal;

        let mut edges = Vec::new();

        // Try parsing as Whirlpool pool account first
        if account_data.len() >= 8 {
            let discriminator = &account_data[0..8];
            const WHIRLPOOL_DISCRIMINATOR: [u8; 8] =
                [0x63, 0x14, 0x93, 0x54, 0x85, 0x35, 0x11, 0x4a];

            if discriminator == WHIRLPOOL_DISCRIMINATOR {
                if let Ok(pool) = WhirlpoolMinimal::try_from_slice(&account_data[8..]) {
                    // Generate EdgeDelta for both directions
                    edges.push(self.create_whirlpool_edge(&pool, slot, write_version, 0)?); // A->B
                    edges.push(self.create_whirlpool_edge(&pool, slot, write_version, 1)?);
                    // B->A
                }
            }
        }

        Ok(edges)
    }

    fn venue_id(&self) -> VenueId {
        VenueId::OrcaWhirlpool
    }
}

impl OrcaWhirlpoolHandler {
    fn create_whirlpool_edge(
        &self,
        pool: &whirlpool_core::types::WhirlpoolMinimal,
        slot: u64,
        write_version: u64,
        direction: u32,
    ) -> anyhow::Result<crate::edge::EdgeDelta> {
        use crate::edge::{edge_delta::State, ClmmState, EdgeDelta};

        let (token_in, token_out) = if direction == 0 {
            (
                pool.token_mint_a.to_bytes().to_vec(),
                pool.token_mint_b.to_bytes().to_vec(),
            )
        } else {
            (
                pool.token_mint_b.to_bytes().to_vec(),
                pool.token_mint_a.to_bytes().to_vec(),
            )
        };

        let clmm_state = ClmmState {
            sqrt_price_x64: pool.sqrt_price.to_le_bytes().to_vec(),
            liquidity: pool.liquidity.to_le_bytes().to_vec(),
            tick_current: pool.tick_current_index,
            tick_spacing: pool.tick_spacing as i32,
            // TODO: Calculate actual depth from tick arrays
            size_to_next_in: pool.liquidity.to_le_bytes().to_vec(),
        };

        Ok(EdgeDelta {
            edge_id: self.generate_edge_id(pool, direction),
            venue_id: self.venue_id() as u32,
            pool_pubkey: b"placeholder".to_vec(), // TODO: Get actual pool pubkey
            token_in,
            token_out,
            direction,
            decimals_in: 6,  // TODO: Get from token metadata
            decimals_out: 6, // TODO: Get from token metadata
            fee_bps_total: pool.fee_rate as u32,
            slot,
            write_version,
            updated_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            marginal_price_q64: pool.sqrt_price.to_le_bytes().to_vec(),
            size_to_next_boundary_in: pool.liquidity.to_le_bytes().to_vec(),
            cu_estimate: 50000, // Estimated CU for Whirlpool swap
            state: Some(State::Clmm(clmm_state)),
        })
    }

    fn generate_edge_id(
        &self,
        pool: &whirlpool_core::types::WhirlpoolMinimal,
        direction: u32,
    ) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        pool.token_mint_a.hash(&mut hasher);
        pool.token_mint_b.hash(&mut hasher);
        direction.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone)]
pub struct RaydiumCpmmHandler;

impl VenueHandler for RaydiumCpmmHandler {
    fn handle_account_update(
        &self,
        account_data: &[u8],
        slot: u64,
        write_version: u64,
    ) -> anyhow::Result<Vec<crate::edge::EdgeDelta>> {
        let mut edges = Vec::new();

        // Try parsing as Raydium AMM pool account
        if account_data.len() >= 8 {
            let discriminator = &account_data[0..8];
            const RAYDIUM_AMM_POOL_DISCRIMINATOR: [u8; 8] =
                [0x5a, 0x7a, 0xc6, 0x3a, 0xdf, 0x8a, 0xf8, 0x1b];

            if discriminator == RAYDIUM_AMM_POOL_DISCRIMINATOR {
                if let Ok(pool) = self.parse_raydium_pool(account_data) {
                    // Generate EdgeDelta for both directions
                    edges.push(self.create_raydium_edge(&pool, slot, write_version, 0)?); // A->B
                    edges.push(self.create_raydium_edge(&pool, slot, write_version, 1)?);
                    // B->A
                }
            }
        }

        Ok(edges)
    }

    fn venue_id(&self) -> VenueId {
        VenueId::RaydiumCpmm
    }
}

#[derive(Debug, Clone)]
pub struct RaydiumPoolMinimal {
    pub token_mint_a: [u8; 32],
    pub token_mint_b: [u8; 32],
    pub reserve_in: u64,
    pub reserve_out: u64,
    pub fee_bps: u16,
}

impl RaydiumCpmmHandler {
    fn parse_raydium_pool(&self, account_data: &[u8]) -> anyhow::Result<RaydiumPoolMinimal> {
        // Skip 8-byte discriminator
        let data = &account_data[8..];

        if data.len() < 32 + 32 + 8 + 8 + 2 {
            return Err(anyhow::anyhow!("Raydium pool account data too short"));
        }

        let mut token_mint_a = [0u8; 32];
        let mut token_mint_b = [0u8; 32];

        token_mint_a.copy_from_slice(&data[0..32]);
        token_mint_b.copy_from_slice(&data[32..64]);

        // Extract reserve amounts and fee - these offsets are estimates, may need adjustment
        let reserve_in = u64::from_le_bytes(data[64..72].try_into()?);
        let reserve_out = u64::from_le_bytes(data[72..80].try_into()?);
        let fee_bps = 25; // Raydium AMM typically uses 25bps

        Ok(RaydiumPoolMinimal {
            token_mint_a,
            token_mint_b,
            reserve_in,
            reserve_out,
            fee_bps,
        })
    }

    fn create_raydium_edge(
        &self,
        pool: &RaydiumPoolMinimal,
        slot: u64,
        write_version: u64,
        direction: u32,
    ) -> anyhow::Result<crate::edge::EdgeDelta> {
        let (token_in, token_out, reserve_in, reserve_out) = match direction {
            0 => (
                pool.token_mint_a,
                pool.token_mint_b,
                pool.reserve_in,
                pool.reserve_out,
            ),
            1 => (
                pool.token_mint_b,
                pool.token_mint_a,
                pool.reserve_out,
                pool.reserve_in,
            ),
            _ => return Err(anyhow::anyhow!("Invalid direction")),
        };

        let edge = crate::edge::EdgeDelta {
            edge_id: self.generate_edge_id(pool, direction),
            venue_id: self.venue_id() as u32,
            pool_pubkey: vec![], // TODO: Add pool pubkey
            token_in: token_in.to_vec(),
            token_out: token_out.to_vec(),
            direction,
            decimals_in: 9,  // TODO: Get from token registry
            decimals_out: 6, // TODO: Get from token registry
            fee_bps_total: pool.fee_bps as u32,
            state: Some(crate::edge::edge_delta::State::Cpmm(
                crate::edge::CpmmState {
                    reserve_in: reserve_in.to_le_bytes().to_vec(),
                    reserve_out: reserve_out.to_le_bytes().to_vec(),
                },
            )),
            slot,
            write_version,
            updated_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            marginal_price_q64: vec![],       // TODO: Calculate price
            size_to_next_boundary_in: vec![], // TODO: Calculate size
            cu_estimate: 50000,
        };

        Ok(edge)
    }

    fn generate_edge_id(&self, pool: &RaydiumPoolMinimal, direction: u32) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        pool.token_mint_a.hash(&mut hasher);
        pool.token_mint_b.hash(&mut hasher);
        direction.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone)]
pub struct MeteoraDlmmHandler;

impl VenueHandler for MeteoraDlmmHandler {
    fn handle_account_update(
        &self,
        account_data: &[u8],
        slot: u64,
        write_version: u64,
    ) -> anyhow::Result<Vec<crate::edge::EdgeDelta>> {
        let mut edges = Vec::new();

        // Try parsing as Meteora DLMM pair account
        if account_data.len() >= 8 {
            let discriminator = &account_data[0..8];
            const METEORA_DLMM_PAIR_DISCRIMINATOR: [u8; 8] =
                [0x5a, 0x64, 0x6d, 0x93, 0x1a, 0x25, 0x5b, 0x8c];

            if discriminator == METEORA_DLMM_PAIR_DISCRIMINATOR {
                if let Ok(pool) = self.parse_meteora_pool(account_data) {
                    // Generate EdgeDelta for both directions
                    edges.push(self.create_meteora_edge(&pool, slot, write_version, 0)?); // A->B
                    edges.push(self.create_meteora_edge(&pool, slot, write_version, 1)?);
                    // B->A
                }
            }
        }

        Ok(edges)
    }

    fn venue_id(&self) -> VenueId {
        VenueId::MeteoraDlmm
    }
}

#[derive(Debug, Clone)]
pub struct MeteoraDlmmPoolMinimal {
    pub token_x_mint: [u8; 32],
    pub token_y_mint: [u8; 32],
    pub reserve_x: u64,
    pub reserve_y: u64,
    pub active_id: i32,
    pub bin_step: u16,
}

impl MeteoraDlmmHandler {
    fn parse_meteora_pool(&self, account_data: &[u8]) -> anyhow::Result<MeteoraDlmmPoolMinimal> {
        // Skip 8-byte discriminator
        let data = &account_data[8..];

        if data.len() < 32 + 32 + 8 + 8 + 4 + 2 {
            return Err(anyhow::anyhow!("Meteora DLMM pool account data too short"));
        }

        let mut token_x_mint = [0u8; 32];
        let mut token_y_mint = [0u8; 32];

        token_x_mint.copy_from_slice(&data[0..32]);
        token_y_mint.copy_from_slice(&data[32..64]);

        // Extract reserve amounts and pool parameters - these offsets are estimates
        let reserve_x = u64::from_le_bytes(data[64..72].try_into()?);
        let reserve_y = u64::from_le_bytes(data[72..80].try_into()?);
        let active_id = i32::from_le_bytes(data[80..84].try_into()?);
        let bin_step = u16::from_le_bytes(data[84..86].try_into()?);

        Ok(MeteoraDlmmPoolMinimal {
            token_x_mint,
            token_y_mint,
            reserve_x,
            reserve_y,
            active_id,
            bin_step,
        })
    }

    fn create_meteora_edge(
        &self,
        pool: &MeteoraDlmmPoolMinimal,
        slot: u64,
        write_version: u64,
        direction: u32,
    ) -> anyhow::Result<crate::edge::EdgeDelta> {
        let (token_in, token_out, reserve_in, _reserve_out) = match direction {
            0 => (
                pool.token_x_mint,
                pool.token_y_mint,
                pool.reserve_x,
                pool.reserve_y,
            ),
            1 => (
                pool.token_y_mint,
                pool.token_x_mint,
                pool.reserve_y,
                pool.reserve_x,
            ),
            _ => return Err(anyhow::anyhow!("Invalid direction")),
        };

        // Calculate fee from bin_step (typical Meteora DLMM fee structure)
        let fee_bps = std::cmp::min(pool.bin_step as u32, 100); // Cap at 100bps, typically 20bps

        let edge = crate::edge::EdgeDelta {
            edge_id: self.generate_edge_id(pool, direction),
            venue_id: self.venue_id() as u32,
            pool_pubkey: vec![], // TODO: Add pool pubkey
            token_in: token_in.to_vec(),
            token_out: token_out.to_vec(),
            direction,
            decimals_in: 9,  // TODO: Get from token registry
            decimals_out: 6, // TODO: Get from token registry
            fee_bps_total: fee_bps,
            state: Some(crate::edge::edge_delta::State::Dlmm(
                crate::edge::DlmmState {
                    active_bin: pool.active_id,
                    bin_price_q64: (pool.bin_step as u128).to_le_bytes().to_vec(), // Simplified bin price
                    depth_hint_in: reserve_in.to_le_bytes().to_vec(),
                },
            )),
            slot,
            write_version,
            updated_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            marginal_price_q64: vec![],       // TODO: Calculate price
            size_to_next_boundary_in: vec![], // TODO: Calculate size
            cu_estimate: 60000,
        };

        Ok(edge)
    }

    fn generate_edge_id(&self, pool: &MeteoraDlmmPoolMinimal, direction: u32) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        pool.token_x_mint.hash(&mut hasher);
        pool.token_y_mint.hash(&mut hasher);
        direction.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone)]
pub struct PhoenixHandler;

impl VenueHandler for PhoenixHandler {
    fn handle_account_update(
        &self,
        account_data: &[u8],
        slot: u64,
        write_version: u64,
    ) -> anyhow::Result<Vec<crate::edge::EdgeDelta>> {
        let mut edges = Vec::new();

        // Try parsing as Phoenix market account
        if account_data.len() >= 8 {
            let discriminator = &account_data[0..8];
            const PHOENIX_MARKET_DISCRIMINATOR: [u8; 8] =
                [0x8b, 0x8f, 0x4e, 0x58, 0x42, 0x01, 0xa9, 0x2c];

            if discriminator == PHOENIX_MARKET_DISCRIMINATOR {
                if let Ok(market) = self.parse_phoenix_market(account_data) {
                    // Generate EdgeDelta for both directions
                    edges.push(self.create_phoenix_edge(&market, slot, write_version, 0)?); // Base->Quote
                    edges.push(self.create_phoenix_edge(&market, slot, write_version, 1)?);
                    // Quote->Base
                }
            }
        }

        Ok(edges)
    }

    fn venue_id(&self) -> VenueId {
        VenueId::Phoenix
    }
}

#[derive(Debug, Clone)]
pub struct PhoenixMarketMinimal {
    pub base_mint: [u8; 32],
    pub quote_mint: [u8; 32],
    pub base_vault_balance: u64,
    pub quote_vault_balance: u64,
    pub tick_size: u64,
}

impl PhoenixHandler {
    fn parse_phoenix_market(&self, account_data: &[u8]) -> anyhow::Result<PhoenixMarketMinimal> {
        // Skip 8-byte discriminator
        let data = &account_data[8..];

        if data.len() < 32 + 32 + 8 + 8 + 8 {
            return Err(anyhow::anyhow!("Phoenix market account data too short"));
        }

        let mut base_mint = [0u8; 32];
        let mut quote_mint = [0u8; 32];

        base_mint.copy_from_slice(&data[0..32]);
        quote_mint.copy_from_slice(&data[32..64]);

        // Extract vault balances and market parameters - these offsets are estimates
        let base_vault_balance = u64::from_le_bytes(data[64..72].try_into()?);
        let quote_vault_balance = u64::from_le_bytes(data[72..80].try_into()?);
        let tick_size = u64::from_le_bytes(data[80..88].try_into()?);

        Ok(PhoenixMarketMinimal {
            base_mint,
            quote_mint,
            base_vault_balance,
            quote_vault_balance,
            tick_size,
        })
    }

    fn create_phoenix_edge(
        &self,
        market: &PhoenixMarketMinimal,
        slot: u64,
        write_version: u64,
        direction: u32,
    ) -> anyhow::Result<crate::edge::EdgeDelta> {
        let (token_in, token_out, reserve_in, reserve_out) = match direction {
            0 => (
                market.base_mint,
                market.quote_mint,
                market.base_vault_balance,
                market.quote_vault_balance,
            ),
            1 => (
                market.quote_mint,
                market.base_mint,
                market.quote_vault_balance,
                market.base_vault_balance,
            ),
            _ => return Err(anyhow::anyhow!("Invalid direction")),
        };

        let edge = crate::edge::EdgeDelta {
            edge_id: self.generate_edge_id(market, direction),
            venue_id: self.venue_id() as u32,
            pool_pubkey: vec![], // TODO: Add market pubkey
            token_in: token_in.to_vec(),
            token_out: token_out.to_vec(),
            direction,
            decimals_in: 9,   // TODO: Get from token registry
            decimals_out: 6,  // TODO: Get from token registry
            fee_bps_total: 5, // Phoenix typically uses 5bps maker fee
            state: Some(crate::edge::edge_delta::State::Clob(
                crate::edge::ClobState {
                    best_bid_px_q64: (reserve_out as u128).to_le_bytes().to_vec(), // Simplified bid price
                    best_bid_size: (reserve_out / 1000).to_le_bytes().to_vec(),
                    best_ask_px_q64: (reserve_in as u128).to_le_bytes().to_vec(), // Simplified ask price
                    best_ask_size: (reserve_in / 1000).to_le_bytes().to_vec(),
                },
            )),
            slot,
            write_version,
            updated_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            marginal_price_q64: vec![],       // TODO: Calculate price
            size_to_next_boundary_in: vec![], // TODO: Calculate size
            cu_estimate: 40000,
        };

        Ok(edge)
    }

    fn generate_edge_id(&self, market: &PhoenixMarketMinimal, direction: u32) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        market.base_mint.hash(&mut hasher);
        market.quote_mint.hash(&mut hasher);
        direction.hash(&mut hasher);
        hasher.finish()
    }
}
