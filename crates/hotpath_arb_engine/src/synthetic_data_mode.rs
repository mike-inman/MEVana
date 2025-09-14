use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};
use log::info;
use crate::orderbook_tracker::{create_arbitrage_event, ARBITRAGE_DETECTED};

/// Synthetic data generator for development when ISP throttles real Geyser data
pub struct SyntheticDataGenerator {
    current_slot: u64,
    opportunity_counter: u64,
}

impl SyntheticDataGenerator {
    pub fn new() -> Self {
        // Start with a realistic current slot number
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Approximate current Solana slot (roughly 2.5 slots per second)
        let current_slot = 366480000 + ((now % 86400) * 25 / 10); // Rough current slot
        
        Self {
            current_slot,
            opportunity_counter: 0,
        }
    }

    /// Generate realistic arbitrage opportunities for dashboard testing
    pub async fn run_synthetic_stream(&mut self) {
        info!("🎭 SYNTHETIC DATA MODE: Generating realistic arbitrage opportunities for dashboard testing");
        info!("📊 This mode simulates live trading data when ISP throttles real Geyser streams");

        loop {
            // Simulate realistic timing - opportunities every 100-500ms
            let delay_ms = 100 + (self.opportunity_counter % 5) * 80;
            sleep(Duration::from_millis(delay_ms)).await;

            // Advance slot (Solana produces ~2.5 slots/second)
            if self.opportunity_counter % 4 == 0 {
                self.current_slot += 1;
            }

            // Generate synthetic arbitrage event
            let profit_bps = self.generate_realistic_profit();
            let venue_pair = self.get_venue_pair();
            let execution_time = self.generate_execution_time();

            let event = create_arbitrage_event(
                &format!("SYNTH-{:06}", self.opportunity_counter),
                self.current_slot,
                &venue_pair.0,
                &venue_pair.1,
                "SOL/USDC",
                profit_bps,
                self.generate_amount_in(),
                self.generate_amount_out(profit_bps),
                execution_time,
                profit_bps > 5, // Success if profit > 5 BPS
                if profit_bps <= 5 { Some("Insufficient profit") } else { None },
            );

            // Push to detection queue (same as real data)
            ARBITRAGE_DETECTED.push(event.clone());

            info!(
                "🎯 SYNTHETIC: {} {}bps {}->{} (slot: {}) - queue: {}",
                event.opportunity_id,
                event.profit_bps,
                event.venue_sell,
                event.venue_buy,
                event.slot,
                ARBITRAGE_DETECTED.len()
            );

            self.opportunity_counter += 1;

            // Simulate occasional bursts of activity
            if self.opportunity_counter % 50 == 0 {
                info!("🚀 SYNTHETIC: Simulating high-activity burst (3 rapid opportunities)");
                for i in 0..3 {
                    sleep(Duration::from_millis(50)).await;
                    let burst_event = create_arbitrage_event(
                        &format!("BURST-{:06}-{}", self.opportunity_counter, i),
                        self.current_slot,
                        "jupiter-v6",
                        "whirlpool",
                        "SOL/USDC",
                        15 + i as i32 * 5, // Higher profits in bursts
                        1.5,
                        1.6,
                        300 + i * 100,
                        true,
                        None,
                    );
                    ARBITRAGE_DETECTED.push(burst_event);
                }
                self.opportunity_counter += 3;
            }
        }
    }

    fn generate_realistic_profit(&self) -> i32 {
        // Simulate realistic profit distribution
        match self.opportunity_counter % 10 {
            0..=3 => 2 + (self.opportunity_counter % 8) as i32,  // Low profits (2-9 BPS)
            4..=6 => 10 + (self.opportunity_counter % 15) as i32, // Medium profits (10-24 BPS)
            7..=8 => 25 + (self.opportunity_counter % 20) as i32, // Good profits (25-44 BPS)
            9 => 50 + (self.opportunity_counter % 30) as i32,     // Excellent profits (50-79 BPS)
            _ => 5,
        }
    }

    fn get_venue_pair(&self) -> (&'static str, &'static str) {
        // Rotate through realistic venue combinations
        match self.opportunity_counter % 6 {
            0 => ("jupiter-v6", "whirlpool"),
            1 => ("whirlpool", "raydium-amm"),
            2 => ("raydium-amm", "meteora-dlmm"),
            3 => ("jupiter-v6", "raydium-amm"),
            4 => ("meteora-dlmm", "whirlpool"),
            5 => ("phoenix-clob", "jupiter-v6"),
            _ => ("jupiter-v6", "whirlpool"),
        }
    }

    fn generate_execution_time(&self) -> u64 {
        // Simulate realistic execution times (300-1500μs)
        300 + (self.opportunity_counter % 1200)
    }

    fn generate_amount_in(&self) -> f64 {
        // Simulate typical trade sizes (0.1 - 5.0 SOL)
        0.1 + ((self.opportunity_counter % 50) as f64 / 10.0)
    }

    fn generate_amount_out(&self, profit_bps: i32) -> f64 {
        let amount_in = self.generate_amount_in();
        // Calculate output based on profit
        amount_in * (1.0 + (profit_bps as f64 / 10000.0))
    }
}

impl Default for SyntheticDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}