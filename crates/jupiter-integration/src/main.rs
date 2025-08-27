use anyhow::Result;
use log::{error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use uuid::Uuid;

// Jupiter API response structures
#[derive(Debug, Deserialize)]
struct QuoteResponse {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    swap_mode: String,
    #[serde(rename = "slippageBps")]
    slippage_bps: u16,
    #[serde(rename = "platformFee")]
    platform_fee: Option<Value>,
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: String,
    #[serde(rename = "routePlan")]
    route_plan: Vec<RoutePlan>,
    #[serde(rename = "contextSlot")]
    context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    time_taken: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RoutePlan {
    #[serde(rename = "swapInfo")]
    swap_info: SwapInfo,
    percent: u8,
}

#[derive(Debug, Deserialize)]
struct SwapInfo {
    #[serde(rename = "ammKey")]
    amm_key: String,
    label: String,
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "feeAmount")]
    fee_amount: String,
    #[serde(rename = "feeMint")]
    fee_mint: String,
}

// Our internal signal structure
#[derive(Debug, Serialize)]
struct JupiterSignal {
    signal_id: String,
    timestamp: u64,
    signal_type: String, // "route_opportunity", "large_swap", "price_impact"
    input_mint: String,
    output_mint: String,
    input_amount: String,
    output_amount: String,
    price_impact_pct: f64,
    route_count: usize,
    venues: Vec<String>,
    context_slot: Option<u64>,
    jupiter_time_taken_ms: Option<f64>,
    our_latency_ms: f64,
}

pub struct JupiterMonitor {
    client: Client,
    producer: FutureProducer,
    base_url: String,
    output_topic: String,
    poll_interval_ms: u64,
    min_amount_threshold: u64,
    min_price_impact: f64,
    api_key: Option<String>,
}

impl JupiterMonitor {
    pub fn new() -> Result<Self> {
        let kafka_broker =
            env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());
        let output_topic =
            env::var("JUPITER_SIGNALS_TOPIC").unwrap_or_else(|_| "jupiter.signals.dev".to_string());
        let base_url =
            env::var("JUPITER_API_URL").unwrap_or_else(|_| "https://api.jup.ag/ultra".to_string());
        let poll_interval_ms = env::var("JUPITER_POLL_INTERVAL_MS")
            .unwrap_or_else(|_| "20000".to_string())
            .parse()
            .unwrap_or(20000);
        let min_amount_threshold = env::var("MIN_AMOUNT_THRESHOLD")
            .unwrap_or_else(|_| "1000000".to_string()) // 1 SOL in lamports
            .parse()
            .unwrap_or(1000000);
        let min_price_impact = env::var("MIN_PRICE_IMPACT_PCT")
            .unwrap_or_else(|_| "0.1".to_string())
            .parse()
            .unwrap_or(0.1);

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_broker)
            .set("message.timeout.ms", "5000")
            .set("linger.ms", "0")
            .set("acks", "1")
            .create()?;

        let timeout_ms = env::var("JUPITER_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(2500);

        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;

        Ok(Self {
            client,
            producer,
            base_url,
            output_topic,
            poll_interval_ms,
            min_amount_threshold,
            min_price_impact,
            api_key: env::var("JUPITER_API_KEY").ok(),
        })
    }

    pub async fn start_monitoring(&self) -> Result<()> {
        info!(
            "Starting Jupiter monitor with {} ms polling interval",
            self.poll_interval_ms
        );

        // Popular token pairs to monitor
        let mut token_pairs = vec![
            // SOL/USDC
            (
                "So11111111111111111111111111111111111111112",
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            ),
            // SOL/USDT
            (
                "So11111111111111111111111111111111111111112",
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
            ),
            // USDC/USDT
            (
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
            ),
            // Add more pairs as needed
        ];

        let mut amounts_to_test = vec![
            1_000_000u64,      // 0.001 SOL
            10_000_000u64,     // 0.01 SOL
            100_000_000u64,    // 0.1 SOL
            1_000_000_000u64,  // 1 SOL
            10_000_000_000u64, // 10 SOL
        ];

        // Throttle calls under rate limiting
        let max_pairs = env::var("MAX_PAIRS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        if token_pairs.len() > max_pairs {
            token_pairs.truncate(max_pairs);
        }

        let max_amounts = env::var("MAX_AMOUNTS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(2);
        if amounts_to_test.len() > max_amounts {
            amounts_to_test.truncate(max_amounts);
        }

        loop {
            let start_time = Instant::now();

            // Monitor each pair with different amounts
            for &(input_mint, output_mint) in &token_pairs {
                for &amount in &amounts_to_test {
                    if amount >= self.min_amount_threshold {
                        if let Err(e) = self
                            .check_pair_and_emit_signal(input_mint, output_mint, amount)
                            .await
                        {
                            warn!("Error checking pair {}/{}: {}", input_mint, output_mint, e);
                        }
                    }
                }
            }

            let elapsed = start_time.elapsed().as_millis() as u64;
            if elapsed < self.poll_interval_ms {
                sleep(Duration::from_millis(self.poll_interval_ms - elapsed)).await;
            }
        }
    }

    async fn check_pair_and_emit_signal(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<()> {
        let start_time = Instant::now();

        let quote_url = format!(
            "{}/v6/quote?inputMint={}&outputMint={}&amount={}&slippageBps=50",
            self.base_url, input_mint, output_mint, amount
        );

        let mut req = self.client.get(&quote_url);
        if let Some(ref key) = self.api_key {
            req = req.header("x-api-key", key);
        }
        match req.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let quote: QuoteResponse = response.json().await?;
                    let our_latency = start_time.elapsed().as_secs_f64() * 1000.0;

                    // Parse price impact
                    let price_impact: f64 = quote.price_impact_pct.parse().unwrap_or(0.0);

                    // Check if this is a significant signal
                    if price_impact >= self.min_price_impact || quote.route_plan.len() > 2 {
                        let signal = JupiterSignal {
                            signal_id: Uuid::new_v4().to_string(),
                            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
                                as u64,
                            signal_type: if price_impact >= 1.0 {
                                "large_price_impact".to_string()
                            } else if quote.route_plan.len() > 2 {
                                "multi_venue_route".to_string()
                            } else {
                                "route_opportunity".to_string()
                            },
                            input_mint: quote.input_mint,
                            output_mint: quote.output_mint,
                            input_amount: quote.in_amount,
                            output_amount: quote.out_amount,
                            price_impact_pct: price_impact,
                            route_count: quote.route_plan.len(),
                            venues: quote
                                .route_plan
                                .iter()
                                .map(|r| r.swap_info.label.clone())
                                .collect(),
                            context_slot: quote.context_slot,
                            jupiter_time_taken_ms: quote.time_taken,
                            our_latency_ms: our_latency,
                        };

                        // Emit signal to Kafka
                        let signal_json = serde_json::to_string(&signal)?;
                        let record = FutureRecord::to(&self.output_topic)
                            .key(&signal.signal_id)
                            .payload(&signal_json);

                        if let Err(e) = self.producer.send(record, Duration::from_millis(100)).await
                        {
                            error!("Failed to send Jupiter signal to Kafka: {:?}", e);
                        } else {
                            info!("Jupiter signal: {} route via {} venues, {:.2}% impact, {:.1}ms latency", 
                                signal.signal_type, signal.route_count, signal.price_impact_pct, signal.our_latency_ms);
                        }
                    }
                } else {
                    warn!(
                        "Jupiter API error: {} for {}/{}",
                        response.status(),
                        input_mint,
                        output_mint
                    );
                }
            }
            Err(e) => {
                warn!(
                    "Jupiter API request failed for {}/{}: {}",
                    input_mint, output_mint, e
                );
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Jupiter Integration Service");

    let monitor = JupiterMonitor::new()?;
    monitor.start_monitoring().await?;

    Ok(())
}
