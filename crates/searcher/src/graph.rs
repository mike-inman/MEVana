use crate::edge::EdgeDelta;
use crate::venue::VenueId;
use ahash::AHashMap;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TokenPair {
    pub token_in: Vec<u8>,
    pub token_out: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub edge_id: u64,
    pub venue_id: VenueId,
    pub pool_pubkey: Vec<u8>,
    pub token_in: Vec<u8>,
    pub token_out: Vec<u8>,
    pub direction: u32,

    // Critical quoting state
    pub marginal_price_q64: u128,
    pub size_to_next_boundary_in: u128,
    pub fee_bps_total: u32,
    pub cu_estimate: u32,

    // Metadata
    pub decimals_in: u32,
    pub decimals_out: u32,
    pub slot: u64,
    pub write_version: u64,
    pub updated_at_ns: u64,
}

impl Edge {
    pub fn from_edge_delta(edge_delta: EdgeDelta) -> Self {
        let marginal_price_q64 = if edge_delta.marginal_price_q64.len() >= 16 {
            u128::from_le_bytes(
                edge_delta.marginal_price_q64[0..16]
                    .try_into()
                    .unwrap_or([0u8; 16]),
            )
        } else {
            0
        };

        let size_to_next_boundary_in = if edge_delta.size_to_next_boundary_in.len() >= 16 {
            u128::from_le_bytes(
                edge_delta.size_to_next_boundary_in[0..16]
                    .try_into()
                    .unwrap_or([0u8; 16]),
            )
        } else {
            0
        };

        Self {
            edge_id: edge_delta.edge_id,
            venue_id: VenueId::from_u32(edge_delta.venue_id).unwrap_or(VenueId::OrcaWhirlpool),
            pool_pubkey: edge_delta.pool_pubkey,
            token_in: edge_delta.token_in,
            token_out: edge_delta.token_out,
            direction: edge_delta.direction,
            marginal_price_q64,
            size_to_next_boundary_in,
            fee_bps_total: edge_delta.fee_bps_total,
            cu_estimate: edge_delta.cu_estimate,
            decimals_in: edge_delta.decimals_in,
            decimals_out: edge_delta.decimals_out,
            slot: edge_delta.slot,
            write_version: edge_delta.write_version,
            updated_at_ns: edge_delta.updated_at_ns,
        }
    }

    pub fn is_stale(&self, max_age_ms: u64) -> bool {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let age_ms = (now_ns - self.updated_at_ns) / 1_000_000;
        age_ms > max_age_ms
    }

    pub fn age_ms(&self) -> u64 {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        (now_ns - self.updated_at_ns) / 1_000_000
    }
}

pub struct EdgeGraph {
    // token_in -> token_out -> Vec<Edge>
    edges_by_input: DashMap<Vec<u8>, AHashMap<Vec<u8>, Vec<Edge>>>,

    // Reverse lookup: token_out -> token_in -> Vec<Edge>
    edges_by_output: DashMap<Vec<u8>, AHashMap<Vec<u8>, Vec<Edge>>>,

    // Direct edge lookup by ID
    edges_by_id: DashMap<u64, Edge>,

    // Statistics
    total_edges: AtomicU64,
    last_cleanup: AtomicU64,
}

impl EdgeGraph {
    pub fn new() -> Self {
        Self {
            edges_by_input: DashMap::new(),
            edges_by_output: DashMap::new(),
            edges_by_id: DashMap::new(),
            total_edges: AtomicU64::new(0),
            last_cleanup: AtomicU64::new(0),
        }
    }

    pub async fn update_edge(&self, edge_delta: EdgeDelta) {
        let edge = Edge::from_edge_delta(edge_delta);
        let edge_id = edge.edge_id;
        let token_in = edge.token_in.clone();
        let token_out = edge.token_out.clone();

        // Check if this is an update to existing edge
        let is_new = !self.edges_by_id.contains_key(&edge_id);

        // Update direct lookup
        self.edges_by_id.insert(edge_id, edge.clone());

        // Update input-based lookup
        let mut input_map = self
            .edges_by_input
            .entry(token_in.clone())
            .or_insert_with(AHashMap::new);
        let output_edges = input_map.entry(token_out.clone()).or_insert_with(Vec::new);

        // Replace or add edge
        if let Some(pos) = output_edges.iter().position(|e| e.edge_id == edge_id) {
            output_edges[pos] = edge.clone();
        } else {
            output_edges.push(edge.clone());
        }

        // Update output-based lookup
        let mut output_map = self
            .edges_by_output
            .entry(token_out)
            .or_insert_with(AHashMap::new);
        let input_edges = output_map.entry(token_in).or_insert_with(Vec::new);

        if let Some(pos) = input_edges.iter().position(|e| e.edge_id == edge_id) {
            input_edges[pos] = edge;
        } else {
            input_edges.push(edge);
        }

        if is_new {
            self.total_edges.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn cleanup_stale_edges(&self, max_age_ms: u64) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let last_cleanup = self.last_cleanup.load(Ordering::Relaxed);

        // Only cleanup every 10 seconds to avoid overhead
        if now_ms - last_cleanup < 10_000 {
            return;
        }

        let mut stale_edge_ids = Vec::new();

        // Find stale edges
        for entry in self.edges_by_id.iter() {
            if entry.value().is_stale(max_age_ms) {
                stale_edge_ids.push(*entry.key());
            }
        }

        // Remove stale edges from all lookups
        for edge_id in stale_edge_ids {
            if let Some((_, edge)) = self.edges_by_id.remove(&edge_id) {
                self.remove_edge_from_lookups(&edge);
                self.total_edges.fetch_sub(1, Ordering::Relaxed);
            }
        }

        self.last_cleanup.store(now_ms, Ordering::Relaxed);
    }

    fn remove_edge_from_lookups(&self, edge: &Edge) {
        // Remove from input-based lookup
        if let Some(mut input_map) = self.edges_by_input.get_mut(&edge.token_in) {
            if let Some(output_edges) = input_map.get_mut(&edge.token_out) {
                output_edges.retain(|e| e.edge_id != edge.edge_id);
                if output_edges.is_empty() {
                    input_map.remove(&edge.token_out);
                }
            }
            if input_map.is_empty() {
                drop(input_map);
                self.edges_by_input.remove(&edge.token_in);
            }
        }

        // Remove from output-based lookup
        if let Some(mut output_map) = self.edges_by_output.get_mut(&edge.token_out) {
            if let Some(input_edges) = output_map.get_mut(&edge.token_in) {
                input_edges.retain(|e| e.edge_id != edge.edge_id);
                if input_edges.is_empty() {
                    output_map.remove(&edge.token_in);
                }
            }
            if output_map.is_empty() {
                drop(output_map);
                self.edges_by_output.remove(&edge.token_out);
            }
        }
    }

    pub fn get_edges_from_token(&self, token_in: &[u8]) -> Option<Vec<(Vec<u8>, Vec<Edge>)>> {
        let input_map = self.edges_by_input.get(token_in)?;
        let mut result = Vec::new();

        for (token_out, edges) in input_map.iter() {
            result.push((token_out.clone(), edges.clone()));
        }

        Some(result)
    }

    pub fn get_edges_to_token(&self, token_out: &[u8]) -> Option<Vec<(Vec<u8>, Vec<Edge>)>> {
        let output_map = self.edges_by_output.get(token_out)?;
        let mut result = Vec::new();

        for (token_in, edges) in output_map.iter() {
            result.push((token_in.clone(), edges.clone()));
        }

        Some(result)
    }

    pub fn get_best_edge(&self, token_in: &[u8], token_out: &[u8]) -> Option<Edge> {
        let input_map = self.edges_by_input.get(token_in)?;
        let edges = input_map.get(token_out)?;

        // Return edge with best marginal price (highest for selling)
        edges.iter().max_by_key(|e| e.marginal_price_q64).cloned()
    }

    pub fn total_edges(&self) -> u64 {
        self.total_edges.load(Ordering::Relaxed)
    }

    pub fn get_all_tokens(&self) -> Vec<Vec<u8>> {
        let mut tokens = std::collections::HashSet::new();

        for entry in self.edges_by_input.iter() {
            tokens.insert(entry.key().clone());
            for (token_out, _) in entry.value().iter() {
                tokens.insert(token_out.clone());
            }
        }

        tokens.into_iter().collect()
    }
}
