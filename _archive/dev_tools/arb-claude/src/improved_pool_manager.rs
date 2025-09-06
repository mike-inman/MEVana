use crate::edge::{EdgeSnapshot, EdgeDelta, ClmmState};
use crate::venue_handlers::VenueId;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use whirlpool_core::tick_window::TickWindow;
use whirlpool_core::types::{TickArray, WhirlpoolMinimal};
use whirlpool_core::quoter::{ClmmQuoter, Direction};

#[derive(Debug)]
pub struct PoolState {
    pub pool: Arc<RwLock<WhirlpoolMinimal>>,
    pub tick_arrays: DashMap<i32, TickArray>,
    pub last_update_slot: AtomicU64,
    pub last_update_write_version: AtomicU64,
    pub last_update_ns: AtomicU64,
    pub edge_cache: Arc<RwLock<Option<(EdgeDelta, EdgeDelta)>>>, // (AtoB, BtoA)
}

impl PoolState {
    pub fn new(pool: WhirlpoolMinimal, slot: u64, write_version: u64) -> Self {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        Self {
            pool: Arc::new(RwLock::new(pool)),
            tick_arrays: DashMap::new(),
            last_update_slot: AtomicU64::new(slot),
            last_update_write_version: AtomicU64::new(write_version),
            last_update_ns: AtomicU64::new(now_ns),
            edge_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Atomically update pool if this is newer data
    pub async fn try_update_pool(&self, new_pool: WhirlpoolMinimal, slot: u64, write_version: u64) -> bool {
        // Atomic check-and-update pattern
        let current_slot = self.last_update_slot.load(Ordering::Acquire);
        let current_write_version = self.last_update_write_version.load(Ordering::Acquire);
        
        // Only update if this is definitively newer
        let should_update = slot > current_slot || 
            (slot == current_slot && write_version > current_write_version);
            
        if should_update {
            // Use compare_exchange to ensure atomicity
            let slot_updated = self.last_update_slot
                .compare_exchange(current_slot, slot, Ordering::Release, Ordering::Relaxed)
                .is_ok();
                
            if slot_updated {
                self.last_update_write_version.store(write_version, Ordering::Release);
                self.last_update_ns.store(
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
                    Ordering::Release
                );
                
                // Update the pool data
                let mut pool_guard = self.pool.write().await;
                *pool_guard = new_pool;
                
                // Invalidate edge cache
                let mut cache_guard = self.edge_cache.write().await;
                *cache_guard = None;
                
                return true;
            }
        }
        
        false
    }

    pub async fn try_update_tick_array(&self, tick_array: TickArray, slot: u64, write_version: u64) -> bool {
        let current_slot = self.last_update_slot.load(Ordering::Acquire);
        
        // Accept tick arrays that are reasonably recent
        if slot >= current_slot.saturating_sub(5) {
            self.tick_arrays.insert(tick_array.start_tick_index, tick_array);
            
            // Invalidate edge cache since liquidity distribution changed
            let mut cache_guard = self.edge_cache.write().await;
            *cache_guard = None;
            
            true
        } else {
            false
        }
    }

    /// Get or compute current edges for this pool
    pub async fn get_current_edges(&self, pool_pubkey: &str) -> Result<(EdgeDelta, EdgeDelta), anyhow::Error> {
        // Check cache first
        {
            let cache_guard = self.edge_cache.read().await;
            if let Some((a_to_b, b_to_a)) = cache_guard.as_ref() {
                return Ok((a_to_b.clone(), b_to_a.clone()));
            }
        }
        
        // Compute new edges
        let pool_guard = self.pool.read().await;
        let edges = self.compute_edges(pool_pubkey, &pool_guard).await?;
        
        // Update cache
        {
            let mut cache_guard = self.edge_cache.write().await;
            *cache_guard = Some(edges.clone());
        }
        
        Ok(edges)
    }

    async fn compute_edges(&self, pool_pubkey: &str, pool: &WhirlpoolMinimal) -> Result<(EdgeDelta, EdgeDelta), anyhow::Error> {
        // Build tick window from current tick arrays
        let tick_window = self.build_tick_window(pool)?;
        
        // Create quoter
        let quoter = ClmmQuoter {
            sqrt_price_x64: pool.sqrt_price,
            liquidity: pool.liquidity,
            tick_current: pool.tick_current_index,
            tick_spacing: pool.tick_spacing as i32,
            total_fee_bps: pool.fee_rate + pool.protocol_fee_rate,
            window: tick_window,
        };

        // Generate both directions
        let edge_a_to_b = self.create_edge_delta(pool_pubkey, pool, &quoter, Direction::AtoB)?;
        let edge_b_to_a = self.create_edge_delta(pool_pubkey, pool, &quoter, Direction::BtoA)?;
        
        Ok((edge_a_to_b, edge_b_to_a))
    }

    fn build_tick_window(&self, pool: &WhirlpoolMinimal) -> Result<TickWindow<'static>, anyhow::Error> {
        // Find the three tick arrays we need based on current tick
        let current_tick = pool.tick_current_index;
        let tick_spacing = pool.tick_spacing as i32;
        
        // Calculate start indices for the three arrays around current tick
        let array_size = 88 * tick_spacing; // Standard Orca tick array size
        let current_array_start = (current_tick / array_size) * array_size;
        
        let lower_start = current_array_start - array_size;
        let upper_start = current_array_start + array_size;
        
        let lower = self.tick_arrays.get(&lower_start).map(|ta| ta.clone());
        let current = self.tick_arrays.get(&current_array_start).map(|ta| ta.clone());
        let upper = self.tick_arrays.get(&upper_start).map(|ta| ta.clone());
        
        Ok(TickWindow { lower, current, upper })
    }

    fn create_edge_delta(
        &self,
        pool_pubkey: &str,
        pool: &WhirlpoolMinimal,
        quoter: &ClmmQuoter,
        direction: Direction,
    ) -> Result<EdgeDelta, anyhow::Error> {
        let size_to_next_in = quoter.size_to_next_tick_in(direction).unwrap_or(0);
        let marginal_price = quoter.marginal_price_q64(direction);
        
        let edge_id = Self::generate_edge_id(pool_pubkey, direction)?;
        
        let (token_in, token_out) = match direction {
            Direction::AtoB => (
                pool.token_mint_a.to_bytes().to_vec(),
                pool.token_mint_b.to_bytes().to_vec(),
            ),
            Direction::BtoA => (
                pool.token_mint_b.to_bytes().to_vec(),
                pool.token_mint_a.to_bytes().to_vec(),
            ),
        };

        let clmm_state = ClmmState {
            sqrt_price_x64: pool.sqrt_price.to_le_bytes().to_vec(),
            liquidity: pool.liquidity.to_le_bytes().to_vec(),
            tick_current: pool.tick_current_index,
            tick_spacing: pool.tick_spacing as i32,
            size_to_next_in: size_to_next_in.to_le_bytes().to_vec(),
        };

        let current_slot = self.last_update_slot.load(Ordering::Acquire);
        let current_write_version = self.last_update_write_version.load(Ordering::Acquire);

        Ok(EdgeDelta {
            edge_id,
            venue_id: VenueId::OrcaWhirlpool as u32,
            pool_pubkey: bs58::decode(pool_pubkey)
                .into_vec()
                .map_err(|e| anyhow::anyhow!("Invalid base58: {}", e))?,
            token_in,
            token_out,
            direction: direction as u32,
            decimals_in: 9,  // TODO: Get from token registry
            decimals_out: 6, // TODO: Get from token registry
            fee_bps_total: (pool.fee_rate + pool.protocol_fee_rate) as u32,
            state: Some(crate::edge::edge_delta::State::Clmm(clmm_state)),
            slot: current_slot,
            write_version: current_write_version,
            updated_at_ns: self.last_update_ns.load(Ordering::Acquire),
            marginal_price_q64: marginal_price.to_le_bytes().to_vec(),
            size_to_next_boundary_in: size_to_next_in.to_le_bytes().to_vec(),
            cu_estimate: 50_000,
        })
    }

    fn generate_edge_id(pool_pubkey: &str, direction: Direction) -> Result<u64, anyhow::Error> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        VenueId::OrcaWhirlpool.hash(&mut hasher);
        pool_pubkey.hash(&mut hasher);
        (direction as u32).hash(&mut hasher);
        Ok(hasher.finish())
    }

    pub fn is_stale(&self, max_age_ms: u64) -> bool {
        let now_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        let last_update_ns = self.last_update_ns.load(Ordering::Acquire);
        let age_ms = (now_ns - last_update_ns) / 1_000_000;
        age_ms > max_age_ms
    }
}

pub struct PoolManager {
    pools: DashMap<String, PoolState>,
    max_pools: usize,
    max_age_ms: u64,
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            max_pools: 10_000,
            max_age_ms: 30_000,
        }
    }

    pub async fn update_pool(&self, pool_pubkey: &str, pool: WhirlpoolMinimal, slot: u64, write_version: u64) -> bool {
        let pool_key = pool_pubkey.to_string();
        
        match self.pools.get(&pool_key) {
            Some(pool_state) => {
                pool_state.try_update_pool(pool, slot, write_version).await
            }
            None => {
                if self.pools.len() < self.max_pools {
                    let pool_state = PoolState::new(pool, slot, write_version);
                    self.pools.insert(pool_key, pool_state);
                    true
                } else {
                    log::warn!("Pool cache full, dropping update for {}", pool_pubkey);
                    false
                }
            }
        }
    }

    pub async fn update_tick_array(&self, pool_pubkey: &str, tick_array: TickArray, slot: u64, write_version: u64) -> bool {
        if let Some(pool_state) = self.pools.get(pool_pubkey) {
            pool_state.try_update_tick_array(tick_array, slot, write_version).await
        } else {
            log::warn!("Received tick array update for unknown pool: {}", pool_pubkey);
            false
        }
    }

    pub async fn get_current_edges(&self, pool_pubkey: &str) -> Option<Result<(EdgeDelta, EdgeDelta), anyhow::Error>> {
        self.pools.get(pool_pubkey)
            .map(|pool_state| pool_state.get_current_edges(pool_pubkey))
    }

    pub async fn create_snapshot(&self) -> EdgeSnapshot {
        let mut all_edges = Vec::new();
        
        for pool_entry in self.pools.iter() {
            let pool_pubkey = pool_entry.key();
            if let Ok((edge_a_to_b, edge_b_to_a)) = pool_entry.value().get_current_edges(pool_pubkey).await {
                all_edges.push(edge_a_to_b);
                all_edges.push(edge_b_to_a);
            }
        }

        let now_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        
        EdgeSnapshot {
            edges: all_edges,
            timestamp_ns: now_ns,
            slot: 0, // TODO: Track latest observed slot across all pools
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
}
