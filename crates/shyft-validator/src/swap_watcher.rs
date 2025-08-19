//! Swap Transaction Watcher
//! This module watches for Orca/Raydium swap transactions and calls Shyft API
//! with a sliding window approach to handle API readiness delays.

use crate::program_ids::{get_dex_type, is_dex_program};
use crate::ShyftValidator;
use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use log::{error, info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Configuration for the swap watcher
#[derive(Debug, Clone)]
pub struct SwapWatcherConfig {
    pub kafka_broker: String,
    pub input_topic: String,
    pub output_topic: String,
    pub group_id: String,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub delay_increment_ms: u64,
    pub batch_size: usize,
}

impl Default for SwapWatcherConfig {
    fn default() -> Self {
        Self {
            kafka_broker: "localhost:9092".to_string(),
            input_topic: "orca-swaps.parsed.v2".to_string(),
            output_topic: "shyft-swaps-parsed".to_string(),
            group_id: "shyft-swap-watcher".to_string(),
            initial_delay_ms: 1000,   // Start with 1 second delay
            max_delay_ms: 30000,      // Max 30 second delay
            delay_increment_ms: 1000, // Increase by 1 second each time
            batch_size: 10,
        }
    }
}

/// Parsed swap transaction from our normalizer
#[derive(Debug, Deserialize, Serialize)]
pub struct ParsedSwap {
    pub signature: String,
    pub dex: String,
    pub program_id: String,
    pub success: bool,
    pub amount_in: u64,
    pub amount_out: u64,
    pub token_in_mint: String,
    pub token_out_mint: String,
    pub pool_id: String,
    pub slot: u64,
    pub block_time_unix: i64,
    pub user_authority: String,
}

/// Swap watcher with sliding window approach
pub struct SwapWatcher {
    config: SwapWatcherConfig,
    validator: ShyftValidator,
    current_delay_ms: u64,
    pending_transactions: VecDeque<(ParsedSwap, Instant)>,
    successful_calls: u32,
    failed_calls: u32,
}

impl SwapWatcher {
    pub fn new(config: SwapWatcherConfig, validator: ShyftValidator) -> Self {
        Self {
            config,
            validator,
            current_delay_ms: 0,
            pending_transactions: VecDeque::new(),
            successful_calls: 0,
            failed_calls: 0,
        }
    }

    /// Start watching for swap transactions
    pub async fn start_watching(&mut self) -> Result<()> {
        info!("🚀 Starting Swap Watcher...");
        info!("📺 Watching topic: {}", self.config.input_topic);
        info!("📤 Output topic: {}", self.config.output_topic);
        info!("⏱️  Initial delay: {}ms", self.config.initial_delay_ms);

        // Create Kafka consumer
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.config.kafka_broker)
            .set("group.id", &self.config.group_id)
            .set("auto.offset.reset", "latest")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .create()
            .context("Failed to create Kafka consumer")?;

        // Subscribe to input topic
        consumer
            .subscribe(&[&self.config.input_topic])
            .context("Failed to subscribe to input topic")?;

        info!("✅ Subscribed to topic: {}", self.config.input_topic);

        // Start processing loop
        self.process_messages(consumer).await?;

        Ok(())
    }

    /// Process incoming messages
    async fn process_messages(&mut self, consumer: StreamConsumer) -> Result<()> {
        let mut message_stream = consumer.stream();

        loop {
            // Process pending transactions with current delay
            self.process_pending_transactions().await;

            // Get next message with timeout
            match tokio::time::timeout(Duration::from_millis(100), message_stream.try_next()).await
            {
                Ok(Ok(Some(msg))) => {
                    if let Some(payload) = msg.payload() {
                        match self.process_swap_message(payload) {
                            Ok(swap) => {
                                info!(
                                    "🔍 New swap detected: {} (DEX: {})",
                                    swap.signature, swap.dex
                                );
                                self.add_pending_transaction(swap);
                            }
                            Err(e) => {
                                warn!("⚠️  Failed to parse swap message: {}", e);
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {
                    // No more messages
                    break;
                }
                Ok(Err(e)) => {
                    error!("❌ Kafka message error: {}", e);
                }
                Err(_) => {
                    // Timeout - continue to process pending transactions
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Process a swap message from Kafka
    fn process_swap_message(&self, payload: &[u8]) -> Result<ParsedSwap> {
        let swap: ParsedSwap =
            serde_json::from_slice(payload).context("Failed to deserialize swap message")?;

        // Validate this is a DEX swap
        if !is_dex_program(&swap.program_id) {
            return Err(anyhow::anyhow!("Not a DEX program: {}", swap.program_id));
        }

        if !swap.success {
            return Err(anyhow::anyhow!("Swap transaction failed"));
        }

        Ok(swap)
    }

    /// Add a transaction to pending queue
    fn add_pending_transaction(&mut self, swap: ParsedSwap) {
        let now = Instant::now();
        self.pending_transactions.push_back((swap, now));

        // Keep queue size manageable
        if self.pending_transactions.len() > self.config.batch_size * 2 {
            self.pending_transactions.pop_front();
        }
    }

    /// Process pending transactions with sliding window delay
    async fn process_pending_transactions(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        for (i, (swap, timestamp)) in self.pending_transactions.iter().enumerate() {
            let age = now.duration_since(*timestamp);
            let required_delay = Duration::from_millis(self.current_delay_ms);

            if age >= required_delay {
                to_remove.push(i);

                // Call Shyft API for this transaction
                match self.call_shyft_api(swap).await {
                    Ok(_) => {
                        success_count += 1;
                        info!(
                            "✅ Successfully processed swap: {} (delay: {}ms)",
                            swap.signature, self.current_delay_ms
                        );
                    }
                    Err(e) => {
                        failure_count += 1;
                        warn!("⚠️  Failed to process swap: {} - {}", swap.signature, e);
                    }
                }
            }
        }

        // Update counters and adjust delay
        self.successful_calls += success_count;
        self.failed_calls += failure_count;

        // If successful, we can try reducing the delay
        if self.successful_calls >= 5 {
            self.reduce_delay();
        }

        // If too many failures, increase the delay
        if self.failed_calls >= 3 {
            self.increase_delay();
        }

        // Remove processed transactions (in reverse order to maintain indices)
        for &index in to_remove.iter().rev() {
            self.pending_transactions.remove(index);
        }

        // Log current status
        if !self.pending_transactions.is_empty() {
            info!(
                "📊 Pending: {}, Delay: {}ms, Success: {}, Failed: {}",
                self.pending_transactions.len(),
                self.current_delay_ms,
                self.successful_calls,
                self.failed_calls
            );
        }
    }

    /// Call Shyft API for a transaction
    async fn call_shyft_api(&self, swap: &ParsedSwap) -> Result<()> {
        info!("🔍 Calling Shyft API for signature: {}", swap.signature);

        // Get the FULL raw JSON response from Shyft API
        let full_shyft_json = self
            .validator
            .get_full_transaction_json(&swap.signature)
            .await
            .context("Failed to call Shyft API")?;

        if let Some(full_json) = full_shyft_json {
            // Send the FULL JSON response to a dedicated topic as requested
            let full_json_output = serde_json::json!({
                "debug_label": "SHYFT_API_COMPLETE_TRANSACTION_PARSED",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "source": "shyft-swap-watcher",
                "transaction_signature": swap.signature,
                "dex_type": get_dex_type(&swap.program_id),
                "full_shyft_response": full_json
            });

            // Send to the FULL JSON topic
            self.validator
                .send_to_kafka(
                    "shyft-full-transactions",
                    &swap.signature,
                    &full_json_output,
                )
                .await?;

            info!("📤 Sent FULL JSON to Kafka topic: shyft-full-transactions");

            // Also fetch and send RAW transaction JSON to its own topic
            if let Some(raw_json) = self
                .validator
                .get_transaction_raw(&swap.signature)
                .await
                .context("Failed to get RAW transaction JSON")?
            {
                let raw_json_output = serde_json::json!({
                    "debug_label": "SHYFT_API_COMPLETE_TRANSACTION_RAW",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "source": "shyft-swap-watcher",
                    "transaction_signature": swap.signature,
                    "dex_type": get_dex_type(&swap.program_id),
                    "full_shyft_response": raw_json
                });

                self.validator
                    .send_to_kafka(
                        "shyft-full-transactions-raw",
                        &swap.signature,
                        &raw_json_output,
                    )
                    .await?;

                info!("📤 Sent RAW JSON to Kafka topic: shyft-full-transactions-raw");
            }

            // Fetch and send RPC getTransaction JSON-RPC response to its own topic
            if let Some(rpc_json) = self
                .validator
                .get_transaction_via_rpc(&swap.signature)
                .await
                .context("Failed to get RPC getTransaction JSON")?
            {
                let rpc_json_output = serde_json::json!({
                    "debug_label": "SHYFT_RPC_GETTRANSACTION",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "source": "shyft-swap-watcher",
                    "transaction_signature": swap.signature,
                    "dex_type": get_dex_type(&swap.program_id),
                    "jsonrpc_response": rpc_json
                });

                self.validator
                    .send_to_kafka(
                        "shyft-rpc-gettransaction",
                        &swap.signature,
                        &rpc_json_output,
                    )
                    .await?;

                info!("📤 Sent RPC JSON to Kafka topic: shyft-rpc-gettransaction");
            }

            // Also send enriched data to the original topic (fixed)
            let shyft_tx = self
                .validator
                .get_transaction_parsed(&swap.signature)
                .await
                .context("Failed to get parsed transaction")?;

            if let Some(shyft_tx) = shyft_tx {
                // Create enriched output with both our data and Shyft data
                let enriched_output = serde_json::json!({
                    "our_parsing": {
                        "signature": swap.signature,
                        "dex": swap.dex,
                        "program_id": swap.program_id,
                        "amount_in": swap.amount_in,
                        "amount_out": swap.amount_out,
                        "token_in_mint": swap.token_in_mint,
                        "token_out_mint": swap.token_out_mint,
                        "pool_id": swap.pool_id,
                        "slot": swap.slot,
                        "block_time_unix": swap.block_time_unix,
                        "user_authority": swap.user_authority,
                    },
                    "shyft_parsing": {
                        "signature": shyft_tx.signature,
                        "slot": shyft_tx.slot,
                        "block_time": shyft_tx.block_time,
                        "fee": shyft_tx.fee,
                        "status": shyft_tx.status,
                        "instructions": shyft_tx.instructions,
                        "account_keys": shyft_tx.account_keys,
                    },
                    "metadata": {
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "source": "shyft-swap-watcher",
                        "dex_type": get_dex_type(&swap.program_id),
                        "processing_delay_ms": self.current_delay_ms,
                    }
                });

                // Send to Kafka
                self.validator
                    .send_to_kafka(&self.config.output_topic, &swap.signature, &enriched_output)
                    .await?;

                info!(
                    "📤 Sent enriched data to Kafka topic: {}",
                    self.config.output_topic
                );
            } else {
                warn!(
                    "⚠️  Shyft API returned no parsed data for signature: {}",
                    swap.signature
                );
            }
        } else {
            warn!(
                "⚠️  Shyft API returned no data for signature: {}",
                swap.signature
            );
        }

        Ok(())
    }

    /// Increase delay for API readiness
    fn increase_delay(&mut self) {
        self.current_delay_ms =
            (self.current_delay_ms + self.config.delay_increment_ms).min(self.config.max_delay_ms);

        info!(
            "⏱️  Increased delay to {}ms (API not ready)",
            self.current_delay_ms
        );

        // Reset counters
        self.successful_calls = 0;
        self.failed_calls = 0;
    }

    /// Reduce delay when API is working well
    fn reduce_delay(&mut self) {
        if self.current_delay_ms > self.config.initial_delay_ms {
            self.current_delay_ms = self
                .current_delay_ms
                .saturating_sub(self.config.delay_increment_ms);
            info!(
                "⏱️  Reduced delay to {}ms (API working well)",
                self.current_delay_ms
            );
        }

        // Reset counters
        self.successful_calls = 0;
        self.failed_calls = 0;
    }
}
