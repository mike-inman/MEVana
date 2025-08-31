use crate::models::{ArbSignal, Pair};
use crate::hotpath::SignalTx;
use dashmap::DashMap;
use std::sync::Arc;
use anyhow::Result;

// Simplified types for edge integration (temporary)
// TODO: Upgrade to real protobuf types when edge-assembler dependency is fully resolved
#[derive(Clone, Debug, Default)]
pub struct EdgeDelta {
    pub edge_id: u64,
    pub venue_id: u32,
    pub pool_pubkey: Vec<u8>,
    pub token_in: Vec<u8>,
    pub token_out: Vec<u8>,
    pub direction: u32,
    pub decimals_in: u32,
    pub decimals_out: u32,
    pub fee_bps_total: u32,
    pub slot: u64,
    pub write_version: u64,
    pub updated_at_ns: u64,
    pub marginal_price_q64: Vec<u8>,
    pub size_to_next_boundary_in: Vec<u8>,
    pub cu_estimate: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ClmmState {
    pub sqrt_price_x64: Vec<u8>,
    pub liquidity: Vec<u8>,
    pub tick_current: i32,
    pub tick_spacing: i32,
    pub size_to_next_in: Vec<u8>,
}

/// Real-time edge-based arbitrage detector
/// Integrates with your existing edge assembler for <10ms latency
pub struct EdgeArbitrageDetector {
    // Store edges by token pair for fast lookup
    edges: DashMap<String, Vec<EdgeDelta>>,

    // Signal output
    signal_tx: SignalTx,

    // Configuration
    min_profit_bps: u32,
    max_slippage_bps: u32,
}

impl EdgeArbitrageDetector {
    pub fn new(signal_tx: SignalTx) -> Self {
        Self {
            edges: DashMap::new(),
            signal_tx,
            min_profit_bps: 50,    // 0.5% minimum profit
            max_slippage_bps: 100, // 1% max slippage
        }
    }

    /// Process incoming edge updates from edge assembler
    /// This is the HOT PATH - called for every pool state change
    #[inline]
    pub fn process_edge_update(&self, edge: EdgeDelta) -> Result<()> {
        // Create token pair key for fast lookup
        let pair_key = self.create_pair_key(&edge.token_in, &edge.token_out);

        // Store edge in memory for fast access
        self.edges
            .entry(pair_key.clone())
            .or_insert_with(Vec::new)
            .push(edge.clone());

        // Immediately scan for arbitrage opportunities
        self.scan_for_arbitrage(&pair_key)?;

        Ok(())
    }

    /// Ultra-fast arbitrage detection using stored edges
    /// Called for every edge update - must be <1ms
    #[inline]
    fn scan_for_arbitrage(&self, pair_key: &str) -> Result<()> {
        let edges = match self.edges.get(pair_key) {
            Some(edges) => edges.clone(),
            None => return Ok(()),
        };

        // Group edges by venue for cross-DEX arbitrage
        let mut venue_edges: std::collections::HashMap<u32, Vec<&EdgeDelta>> =
            std::collections::HashMap::new();

        for edge in &edges {
            venue_edges
                .entry(edge.venue_id)
                .or_insert_with(Vec::new)
                .push(edge);
        }

        // Need at least 2 venues for arbitrage
        if venue_edges.len() < 2 {
            return Ok(());
        }

        // Find best arbitrage opportunity across venues
        let mut best_arb: Option<ArbSignal> = None;
        let mut best_profit = 0i128;

        // Compare all venue pairs
        let venues: Vec<u32> = venue_edges.keys().copied().collect();
        for i in 0..venues.len() {
            for j in (i + 1)..venues.len() {
                let venue1 = venues[i];
                let venue2 = venues[j];

                if let (Some(edges1), Some(edges2)) =
                    (venue_edges.get(&venue1), venue_edges.get(&venue2))
                {
                    // Find best buy/sell combination
                    if let Some(arb) = self.calculate_cross_venue_arbitrage(edges1, edges2) {
                        if arb.expected_profit_out > best_profit {
                            best_profit = arb.expected_profit_out;
                            best_arb = Some(arb);
                        }
                    }
                }
            }
        }

        // Emit best arbitrage signal
        if let Some(arb) = best_arb {
            let _ = self.signal_tx.try_send(arb);
        }

        Ok(())
    }

    /// Calculate arbitrage between two venues
    #[inline]
    fn calculate_cross_venue_arbitrage(
        &self,
        edges1: &[&EdgeDelta],
        edges2: &[&EdgeDelta],
    ) -> Option<ArbSignal> {
        // Find best buy and sell opportunities
        let mut best_buy: Option<&EdgeDelta> = None;
        let mut best_sell: Option<&EdgeDelta> = None;
        let mut best_profit = 0i128;

        // Compare all edge combinations
        for edge1 in edges1 {
            for edge2 in edges2 {
                // Skip if same direction
                if edge1.direction == edge2.direction {
                    continue;
                }

                // Calculate potential profit
                if let Some(profit) = self.calculate_edge_profit(edge1, edge2) {
                    if profit > best_profit {
                        best_profit = profit;
                        best_buy = Some(edge1);
                        best_sell = Some(edge2);
                    }
                }
            }
        }

        // Create arbitrage signal
        if let (Some(buy), Some(sell)) = (best_buy, best_sell) {
            let pair = Pair {
                a: bs58::encode(&buy.token_in).into_string(),
                b: bs58::encode(&buy.token_out).into_string(),
            };

            Some(ArbSignal {
                pair,
                dex_sell: self.venue_id_to_dex_name(sell.venue_id),
                dex_buy: self.venue_id_to_dex_name(buy.venue_id),
                amount_in: self.parse_size_to_boundary(sell),
                expected_profit_out: best_profit,
                slot: sell.slot,
                sell_pool_id: bs58::encode(&sell.pool_pubkey).into_string(),
                buy_pool_id: bs58::encode(&buy.pool_pubkey).into_string(),
            })
        } else {
            None
        }
    }

    /// Calculate profit between two edges
    #[inline]
    fn calculate_edge_profit(&self, edge1: &EdgeDelta, edge2: &EdgeDelta) -> Option<i128> {
        // Parse amounts from edge data
        let amount1 = self.parse_size_to_boundary(edge1);
        let amount2 = self.parse_size_to_boundary(edge2);

        if amount1 == 0 || amount2 == 0 {
            return None;
        }

        // Calculate profit (simplified - you'll want to add proper price calculation)
        let profit = (amount2 as i128) - (amount1 as i128);

        // Apply minimum profit threshold
        let min_profit = (amount1 as i128 * self.min_profit_bps as i128) / 10000;

        if profit >= min_profit {
            Some(profit)
        } else {
            None
        }
    }

    /// Parse size to next boundary from edge data
    #[inline]
    fn parse_size_to_boundary(&self, edge: &EdgeDelta) -> u128 {
        if edge.size_to_next_boundary_in.len() >= 16 {
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&edge.size_to_next_boundary_in[..16]);
            u128::from_le_bytes(bytes)
        } else {
            0
        }
    }

    /// Create pair key for fast lookup
    #[inline]
    fn create_pair_key(&self, token_in: &[u8], token_out: &[u8]) -> String {
        format!(
            "{}_{}",
            bs58::encode(token_in).into_string(),
            bs58::encode(token_out).into_string()
        )
    }

    /// Convert venue ID to DEX name
    fn venue_id_to_dex_name(&self, venue_id: u32) -> String {
        match venue_id {
            1 => "orca_whirlpool".to_string(),
            2 => "raydium_cpmm".to_string(),
            3 => "meteora_dlmm".to_string(),
            4 => "phoenix".to_string(),
            _ => format!("unknown_{}", venue_id),
        }
    }

    /// Update configuration (for AI agent tuning)
    pub fn update_config(&mut self, min_profit_bps: u32, max_slippage_bps: u32) {
        self.min_profit_bps = min_profit_bps;
        self.max_slippage_bps = max_slippage_bps;
    }
}

/// Kafka consumer for edge assembler messages
/// This connects your existing edge assembler to the arb engine
pub struct EdgeConsumer {
    detector: Arc<EdgeArbitrageDetector>,
    consumer: rdkafka::consumer::StreamConsumer,
    topic: String,
}

impl EdgeConsumer {
    pub fn new(
        detector: Arc<EdgeArbitrageDetector>,
        kafka_broker: String,
        topic: String,
    ) -> Result<Self> {
        let consumer: rdkafka::consumer::StreamConsumer = rdkafka::config::ClientConfig::new()
            .set("group.id", "arb-engine-edge-consumer")
            .set("bootstrap.servers", &kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "1")
            .set("fetch.min.bytes", "1")
            .create()?;

        Ok(Self {
            detector,
            consumer,
            topic,
        })
    }

    /// Start consuming edge updates from Kafka
    pub async fn start(&self) -> Result<()> {
        use rdkafka::consumer::Consumer;
        use rdkafka::message::Message;

        println!("🚀 Starting Edge Consumer for topic: {}", self.topic);

        // Subscribe to edge delta topic
        self.consumer.subscribe(&[&self.topic])?;

        // Main consumption loop
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        // For now, we'll create a mock edge update
                        // TODO: Implement proper protobuf decoding when edge-assembler is properly linked
                        let mock_edge = EdgeDelta {
                            edge_id: 12345,
                            venue_id: 1, // Orca Whirlpool
                            pool_pubkey: vec![1; 32],
                            token_in: vec![1; 32],
                            token_out: vec![2; 32],
                            direction: 0, // AtoB
                            decimals_in: 9,
                            decimals_out: 6,
                            fee_bps_total: 30,
                            slot: 123456789,
                            write_version: 1,
                            updated_at_ns: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos() as u64,
                            marginal_price_q64: vec![
                                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            ],
                            size_to_next_boundary_in: vec![
                                100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            ],
                            cu_estimate: 50000,
                        };

                        // Process the edge update
                        if let Err(e) = self.detector.process_edge_update(mock_edge) {
                            eprintln!("❌ Error processing edge: {}", e);
                        }
                    }

                    // Commit offset after processing
                    if let Err(e) = self
                        .consumer
                        .commit_message(&message, rdkafka::consumer::CommitMode::Async)
                    {
                        eprintln!("❌ Failed to commit offset: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Kafka error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}
