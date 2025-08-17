use anyhow::Result;
use dotenv::dotenv;
use env_logger;
use log::{info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
struct NormalizedSwap {
    slot: u64,
    block_time_unix: u64,
    signature: String,
    ix_index: u8,
    program_id: String,
    dex: String,
    pool_id: String,
    swap_kind: String,
    token_in_mint: String,
    token_out_mint: String,
    amount_in: u64,
    amount_out: u64,
    fee_bps_estimate: Option<u16>,
    price: Option<f64>,
    tick_index: Option<i32>,
    sqrt_price_x64: Option<String>,
    user_authority: String,
    success: bool,
    quality: Option<Quality>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Quality {
    idl_decoded: bool,
    from_profile: bool,
    amounts_from_meta: bool,
    pool_resolved: bool,
}

#[derive(Debug, Serialize)]
struct ArbOpportunity {
    pool_id: String,
    token_in: String,
    token_out: String,
    amount_in_ui: f64,
    amount_out_ui: f64,
    price: f64,
    net_pnl_usd: f64,
    timestamp: u64,
}

struct ArbTestMode {
    kafka_consumer: StreamConsumer,
    redis_client: redis::Client,
    opportunities: Vec<ArbOpportunity>,
    total_swaps: u64,
    total_opportunities: u64,
}

impl ArbTestMode {
    async fn new(broker: &str, topic: &str, redis_url: &str) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", broker)
            .set("group.id", "arb-test-mode-group")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "latest")
            .create()?;
        
        consumer.subscribe(&[topic])?;
        
        let redis_client = redis::Client::open(redis_url)?;
        
        Ok(Self {
            kafka_consumer: consumer,
            redis_client,
            opportunities: Vec::new(),
            total_swaps: 0,
            total_opportunities: 0,
        })
    }
    
    async fn run(&mut self) -> Result<()> {
        info!("arb-test-mode: starting arbitrage opportunity detection");
        
        loop {
            let msg = self.kafka_consumer.recv().await?;
            if let Some(payload) = msg.payload_view::<str>().and_then(|r| r.ok()) {
                let payload = payload.to_string(); // Clone the payload to avoid borrow issues
                self.process_swap(&payload).await?;
            }
        }
    }
    
    async fn process_swap(&mut self, payload: &str) -> Result<()> {
        let start = Instant::now();
        
        match serde_json::from_str::<NormalizedSwap>(payload) {
            Ok(swap) => {
                self.total_swaps += 1;
                
                // Basic arbitrage detection: look for large price movements
                if let Some(price) = swap.price {
                    let amount_in_ui = self.parse_amount(swap.amount_in, &swap.token_in_mint).await?;
                    let amount_out_ui = self.parse_amount(swap.amount_out, &swap.token_out_mint).await?;
                    
                    // Simple opportunity detection: large trades with significant price impact
                    let trade_size_usd = amount_in_ui * price; // Rough USD estimate
                    let price_impact = self.calculate_price_impact(&swap).await?;
                    
                    if trade_size_usd > 1000.0 && price_impact.abs() > 0.01 {
                        let net_pnl = self.estimate_net_pnl(&swap, price_impact).await?;
                        
                        if net_pnl > 0.0 {
                            let opportunity = ArbOpportunity {
                                pool_id: swap.pool_id.clone(),
                                token_in: swap.token_in_mint.clone(),
                                token_out: swap.token_out_mint.clone(),
                                amount_in_ui,
                                amount_out_ui,
                                price,
                                net_pnl_usd: net_pnl,
                                timestamp: swap.block_time_unix,
                            };
                            
                            self.opportunities.push(opportunity);
                            self.total_opportunities += 1;
                            
                            info!("WOULD TRADE: pool={} size=${:.2} impact={:.3}% pnl=${:.2}", 
                                swap.pool_id, trade_size_usd, price_impact * 100.0, net_pnl);
                        }
                    }
                }
                
                if self.total_swaps % 100 == 0 {
                    let latency = start.elapsed();
                    info!("processed {} swaps, {} opportunities, latency: {:?}", 
                        self.total_swaps, self.total_opportunities, latency);
                }
            }
            Err(e) => {
                warn!("failed to parse swap: {}", e);
            }
        }
        
        Ok(())
    }
    
    async fn parse_amount(&self, raw_amount: u64, mint: &str) -> Result<f64> {
        // Get mint decimals from Redis cache
        let mut conn = self.redis_client.get_multiplexed_tokio_connection().await?;
        let decimals: Option<u8> = conn.hget(&format!("dex:mint:{}", mint), "decimals").await?;
        
        let decimals = decimals.unwrap_or(9) as u32;
        let raw: u128 = raw_amount.into();
        let amount_ui = raw as f64 / 10f64.powi(decimals as i32);
        
        Ok(amount_ui)
    }
    
    async fn calculate_price_impact(&self, swap: &NormalizedSwap) -> Result<f64> {
        // For now, use a simple heuristic based on trade size
        let amount_in_ui = self.parse_amount(swap.amount_in, &swap.token_in_mint).await?;
        let price = swap.price.unwrap_or(1.0);
        let trade_size_usd = amount_in_ui * price;
        
        // Rough impact estimate: larger trades = more impact
        let impact = (trade_size_usd / 10000.0).min(0.1); // Cap at 10%
        
        Ok(impact)
    }
    
    async fn estimate_net_pnl(&self, swap: &NormalizedSwap, price_impact: f64) -> Result<f64> {
        let amount_in_ui = self.parse_amount(swap.amount_in, &swap.token_in_mint).await?;
        let price = swap.price.unwrap_or(1.0);
        let trade_size_usd = amount_in_ui * price;
        
        // Estimate fees (DEX + Jito tip)
        let dex_fee_bps = swap.fee_bps_estimate.unwrap_or(100) as f64 / 10000.0;
        let dex_fee = trade_size_usd * dex_fee_bps;
        let jito_tip = 0.001; // $0.001 tip for now
        
        // Estimate profit from price impact
        let profit = trade_size_usd * price_impact * 0.5; // Assume 50% of impact is captured
        
        let net_pnl = profit - dex_fee - jito_tip;
        
        Ok(net_pnl)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    
    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let topic = std::env::var("IN_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed.v2".into());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379/0".into());
    
    info!("arb-test-mode: starting with broker={}, topic={}, redis={}", broker, topic, redis_url);
    
    let mut arb_test = ArbTestMode::new(&broker, &topic, &redis_url).await?;
    arb_test.run().await?;
    
    Ok(())
}
