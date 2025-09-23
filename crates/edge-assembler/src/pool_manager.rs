#![allow(dead_code)]

use crate::edge::EdgeSnapshot;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use whirlpool_core::tick_window::TickWindow;
use whirlpool_core::types::{TickArray, WhirlpoolMinimal};

#[derive(Debug)]
pub struct PoolState {
    pub pool: WhirlpoolMinimal,
    pub tick_arrays: DashMap<i32, TickArray>, // start_tick_index -> TickArray
    pub tick_window: TickWindow<'static>,     // Will need to manage lifetime properly
    pub last_update_slot: AtomicU64,
    pub last_update_ns: AtomicU64,
}

impl PoolState {
    pub fn new(pool: WhirlpoolMinimal, slot: u64) -> Self {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        Self {
            pool,
            tick_arrays: DashMap::new(),
            tick_window: TickWindow {
                lower: None,
                current: None,
                upper: None,
            }, // TODO: Properly construct from tick arrays
            last_update_slot: AtomicU64::new(slot),
            last_update_ns: AtomicU64::new(now_ns),
        }
    }

    pub fn update_pool(&self, _pool: WhirlpoolMinimal, slot: u64, write_version: u64) -> bool {
        let current_slot = self.last_update_slot.load(Ordering::Acquire);

        // Only update if this is newer data
        if slot > current_slot || (slot == current_slot && write_version > 0) {
            // In a real implementation, we'd need proper synchronization here
            // For now, we'll accept some race conditions for simplicity
            self.last_update_slot.store(slot, Ordering::Release);
            self.last_update_ns.store(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                Ordering::Release,
            );
            true
        } else {
            false
        }
    }

    pub fn update_tick_array(&self, tick_array: TickArray, slot: u64, _write_version: u64) -> bool {
        let current_slot = self.last_update_slot.load(Ordering::Acquire);

        // Update tick array if slot is recent enough
        if slot >= current_slot.saturating_sub(2) {
            self.tick_arrays
                .insert(tick_array.start_tick_index, tick_array);

            // TODO: Rebuild tick_window from updated tick_arrays
            // This requires proper lifetime management and is a complex operation

            true
        } else {
            false
        }
    }

    pub fn is_stale(&self, max_age_ms: u64) -> bool {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let last_update_ns = self.last_update_ns.load(Ordering::Acquire);
        let age_ms = (now_ns - last_update_ns) / 1_000_000;

        age_ms > max_age_ms
    }
}

pub struct PoolManager {
    pools: DashMap<String, PoolState>, // pool_pubkey -> PoolState
    max_pools: usize,
    max_age_ms: u64,
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            max_pools: 10_000,  // Limit memory usage
            max_age_ms: 30_000, // 30 seconds max age
        }
    }

    pub fn update_pool(
        &self,
        pool_pubkey: &str,
        pool: WhirlpoolMinimal,
        slot: u64,
        write_version: u64,
    ) {
        let pool_key = pool_pubkey.to_string();

        match self.pools.get(&pool_key) {
            Some(pool_state) => {
                // Update existing pool
                pool_state.update_pool(pool, slot, write_version);
            }
            None => {
                // Insert new pool
                if self.pools.len() < self.max_pools {
                    let pool_state = PoolState::new(pool, slot);
                    self.pools.insert(pool_key, pool_state);
                } else {
                    // TODO: Implement LRU eviction or prioritization
                    log::warn!("Pool cache full, dropping update for {}", pool_pubkey);
                }
            }
        }
    }

    pub fn update_tick_array(
        &self,
        pool_pubkey: &str,
        tick_array: TickArray,
        slot: u64,
        write_version: u64,
    ) {
        if let Some(pool_state) = self.pools.get(pool_pubkey) {
            pool_state.update_tick_array(tick_array, slot, write_version);
        } else {
            log::warn!(
                "Received tick array update for unknown pool: {}",
                pool_pubkey
            );
        }
    }

    pub fn get_pool(
        &self,
        pool_pubkey: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, String, PoolState>> {
        self.pools.get(pool_pubkey)
    }

    pub fn create_snapshot(&self) -> EdgeSnapshot {
        // For now, return empty snapshot
        // In a real implementation, we'd convert all current pool states to EdgeDeltas
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        EdgeSnapshot {
            edges: vec![], // TODO: Generate EdgeDeltas from all pools
            timestamp_ns: now_ns,
            slot: 0, // TODO: Track latest observed slot
        }
    }

    pub fn cleanup_stale_pools(&self) {
        let stale_keys: Vec<String> = self
            .pools
            .iter()
            .filter(|entry| entry.value().is_stale(self.max_age_ms))
            .map(|entry| entry.key().clone())
            .collect();

        for key in stale_keys {
            self.pools.remove(&key);
            log::info!("Removed stale pool: {}", key);
        }
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}
