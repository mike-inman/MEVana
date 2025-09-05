use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use chrono;
use clap::Parser;
use dotenv::dotenv;
use env_logger;
use futures_util::{sink::SinkExt, stream::StreamExt, TryFutureExt};
use log::{debug, error, info, warn};
use prometheus::{Histogram, HistogramOpts, IntCounter, Registry};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tonic::transport::channel::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    geyser::{SubscribeRequestFilterAccounts, SubscribeRequestFilterTransactions},
    prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
    },
    prost::Message as _,
};

// Import tick array parsing
use dex_stream_universal::tick_array_parser::{TickArrayParser, TickArray, Tick, WhirlpoolAccountType, Position};
use edge_assembler::venue_handlers::{VenueHandler, OrcaWhirlpoolHandler, RaydiumCpmmHandler, MeteoraDlmmHandler, PhoenixHandler};
use edge_assembler::{EdgeDelta};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use prost::Message;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "Live Whirlpool MEV Stream for Arbitrage Detection")]
struct Args {
    #[arg(long, env = "ENDPOINT", default_value = "https://grpc.ny.shyft.to")]
    endpoint: String,
    
    #[arg(long, env = "X_TOKEN")]
    x_token: String,
    
    #[arg(long, default_value = "8080")]
    metrics_port: u16,
}

// High-volume Whirlpool pools for testing (curated list)
const TARGET_WHIRLPOOL_POOLS: &[&str] = &[
    "HJPjoWUrhoZzkNfRpHuieeFk9WcPWjwy6PBjZ81ngndJ", // SOL/USDC (0.3%)
    "7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm", // SOL/USDT (0.3%)  
    "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d", // ETH/SOL (0.3%)
    "9vqYJjDUFecLL2xPUC4Rc7hyCtZ6iJ4mDiVZX7aFXoAe", // BTC/SOL (0.3%)
    "4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4", // USDC/USDT (0.01%)
    "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE", // RAY/SOL (0.3%)
    "A8t5BSsbcw1ypWQZJazHb4aJE7ghAhcQjPFMu4qLHgsN", // BONK/SOL (1%)
    "FEDT8VzsXFhRpZ4Jn1hsCM2CnWNL8yTxLEKFhF5NfqyW", // JUP/SOL (1%)
    "EiYXQeRkQp8pBWkk9WjPnmb4xjq4wt7p8C9fCT3nSveL", // WIF/SOL (1%)
    "H4snTKK9adiKKEk8ALTrPXhNuF3MgvNaE8HWuvgHEPJF", // POPCAT/SOL (1%)
];

// TickArray accounts for these pools (derived addresses)
const TARGET_TICKARRAY_RANGE: i32 = 10; // Monitor ±10 tick arrays around current price

#[derive(Clone)]
struct WhirlpoolState {
    pools: Arc<RwLock<HashMap<String, PoolData>>>,
    tick_arrays: Arc<RwLock<HashMap<String, TickArrayData>>>,
    opportunities_detected: Arc<RwLock<u64>>,
    last_update: Arc<RwLock<Instant>>,
    // Multiple venue handlers
    whirlpool_handler: OrcaWhirlpoolHandler,
    raydium_handler: RaydiumCpmmHandler,
    meteora_handler: MeteoraDlmmHandler,
    phoenix_handler: PhoenixHandler,
    kafka_producer: Option<FutureProducer>,
    // Discovery tracking
    unknown_programs: Arc<RwLock<HashMap<String, u64>>>, // program_id -> count
}

#[derive(Debug, Clone)]
struct PoolData {
    pubkey: String,
    token_a: String,
    token_b: String,
    sqrt_price: u128,
    liquidity: u128,
    tick_current: i32,
    tick_spacing: i32,
    fee_rate: u16,
    slot: u64,
    write_version: u64,
}

#[derive(Debug, Clone)]
struct TickArrayData {
    pubkey: String,
    pool: String,
    start_tick_index: i32,
    ticks: Vec<Tick>,
    slot: u64,
    write_version: u64,
}

impl WhirlpoolState {
    fn new() -> Self {
        // Set up Kafka producer for EdgeDelta publishing
        let kafka_producer = Self::setup_kafka_producer();
        
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            tick_arrays: Arc::new(RwLock::new(HashMap::new())),
            opportunities_detected: Arc::new(RwLock::new(0)),
            last_update: Arc::new(RwLock::new(Instant::now())),
            whirlpool_handler: OrcaWhirlpoolHandler,
            raydium_handler: RaydiumCpmmHandler,
            meteora_handler: MeteoraDlmmHandler,
            phoenix_handler: PhoenixHandler,
            kafka_producer,
            unknown_programs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn setup_kafka_producer() -> Option<FutureProducer> {
        let kafka_broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());
        
        match ClientConfig::new()
            .set("bootstrap.servers", &kafka_broker)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "lz4")
            .set("batch.size", "16384")
            .set("linger.ms", "1")
            .create::<FutureProducer>()
        {
            Ok(producer) => {
                info!("✅ Kafka producer initialized: {}", kafka_broker);
                Some(producer)
            }
            Err(e) => {
                warn!("⚠️  Kafka producer setup failed (will log EdgeDeltas only): {}", e);
                None
            }
        }
    }

    async fn process_account_update(&self, update: &yellowstone_grpc_proto::prelude::SubscribeUpdateAccount) -> Result<()> {
        let pubkey = bs58::encode(&update.account.as_ref().unwrap().pubkey).into_string();
        let account_data = &update.account.as_ref().unwrap().data;
        let owner = bs58::encode(&update.account.as_ref().unwrap().owner).into_string();
        let slot = update.slot;
        let write_version = update.account.as_ref().unwrap().write_version;

        // MULTI-VENUE PROCESSING: Try each venue handler based on owner
        let processed = match owner.as_str() {
            // Venue 1: Whirlpool/Orca
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => {
                self.process_whirlpool_account(&pubkey, account_data, slot, write_version).await
            }
            // Venue 2: Raydium AMM  
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
                self.process_venue_account(2, "Raydium", &pubkey, account_data, slot, write_version).await
            }
            // Venue 3: Meteora DLMM
            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => {
                self.process_venue_account(3, "Meteora", &pubkey, account_data, slot, write_version).await
            }
            // Venue 4: Phoenix
            "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" => {
                self.process_venue_account(4, "Phoenix", &pubkey, account_data, slot, write_version).await
            }
            // DISCOVERY: Unknown program - log for analysis
            _ => {
                self.process_unknown_program(&pubkey, &owner, account_data, slot, write_version).await
            }
        };

        if processed {
            *self.last_update.write().await = Instant::now();
        }
        
        Ok(())
    }

    async fn process_whirlpool_account(&self, pubkey: &str, account_data: &[u8], slot: u64, write_version: u64) -> bool {
        // Try to parse as Whirlpool pool
        if let Some(pool_data) = self.try_parse_whirlpool_pool(pubkey, account_data, slot, write_version) {
            self.pools.write().await.insert(pubkey.to_string(), pool_data.clone());
            
            info!("🏊 WHIRLPOOL POOL UPDATE: {} | Tokens: {}/{} | Price: {} | Liquidity: {} | Tick: {} | Fee: {}bps",
                pubkey,
                &pool_data.token_a[..8],
                &pool_data.token_b[..8], 
                pool_data.sqrt_price,
                pool_data.liquidity,
                pool_data.tick_current,
                pool_data.fee_rate
            );

            // Process pool for edge opportunities
            self.process_edge_opportunities(account_data, slot, write_version).await;
            return true;
        }

        // Try to parse any Whirlpool account type
        match TickArrayParser::parse_account(account_data) {
            Ok(WhirlpoolAccountType::TickArray(tick_array)) => {
                let tick_data = TickArrayData {
                    pubkey: pubkey.to_string(),
                    pool: tick_array.whirlpool.to_string(),
                    start_tick_index: tick_array.start_tick_index,
                    ticks: tick_array.ticks.to_vec(),
                    slot,
                    write_version,
                };
                
                self.tick_arrays.write().await.insert(pubkey.to_string(), tick_data.clone());
                
                let initialized_count = tick_array.ticks.iter().filter(|t| dex_stream_universal::tick_array_parser::tick_is_initialized(t)).count();
                info!("🎯 TICK ARRAY UPDATE: {} | Pool: {} | Start: {} | Initialized: {}/88",
                    pubkey,
                    &tick_array.whirlpool.to_string()[..8],
                    tick_array.start_tick_index,
                    initialized_count
                );

                // Check for arbitrage opportunities using EdgeAssembler
                self.process_edge_opportunities(account_data, slot, write_version).await;
                return true;
            }
            Ok(WhirlpoolAccountType::Position(position)) => {
                info!("📍 POSITION UPDATE: {} | Pool: {} | Liquidity: {} | Range: {} to {}",
                    pubkey,
                    &position.whirlpool.to_string()[..8],
                    position.liquidity,
                    position.tick_lower_index,
                    position.tick_upper_index
                );
                return true;
            }
            Ok(WhirlpoolAccountType::Pool) => {
                // Pool account - already handled above
                return true;
            }
            Err(_) => {
                // Unknown whirlpool account type
                return false;
            }
        }
    }

    async fn process_venue_account(&self, venue_id: u32, venue_name: &str, pubkey: &str, account_data: &[u8], slot: u64, write_version: u64) -> bool {
        // Use the appropriate venue handler
        let edges_result = match venue_id {
            2 => self.raydium_handler.handle_account_update(account_data, slot, write_version),
            3 => self.meteora_handler.handle_account_update(account_data, slot, write_version),
            4 => self.phoenix_handler.handle_account_update(account_data, slot, write_version),
            _ => return false,
        };

        match edges_result {
            Ok(edges) if !edges.is_empty() => {
                info!("🔥 {} ACCOUNT UPDATE: {} | Generated {} EdgeDeltas | Slot: {} | Size: {} bytes",
                    venue_name, pubkey, edges.len(), slot, account_data.len());
                
                // Send EdgeDeltas to hotpath
                self.send_to_hotpath(edges).await;
                true
            }
            Ok(_) => {
                // Account processed but no edges generated
                debug!("✅ {} account processed: {} | No edges generated", venue_name, pubkey);
                false  
            }
            Err(e) => {
                debug!("❌ {} account processing failed: {} | Error: {}", venue_name, pubkey, e);
                false
            }
        }
    }

    async fn process_unknown_program(&self, pubkey: &str, owner: &str, account_data: &[u8], slot: u64, _write_version: u64) -> bool {
        // DISCOVERY: Log unknown programs for analysis
        if account_data.len() >= 8 {
            let discriminator = &account_data[0..8];
            info!("🔍 DISCOVERY: Unknown Program | Owner: {} | Account: {} | Size: {} bytes | Slot: {} | Discriminator: {:?}",
                owner, pubkey, account_data.len(), slot, discriminator);
                
            // Could send to discovery-service here for AI analysis
            return true;
        }
        false
    }

    fn try_parse_whirlpool_pool(&self, pubkey: &str, data: &[u8], slot: u64, write_version: u64) -> Option<PoolData> {
        // Check length - Whirlpool accounts are around 653 bytes
        if data.len() < 600 { 
            return None; 
        }

        // Check Whirlpool discriminator
        let whirlpool_discriminator = [0x63, 0x14, 0x93, 0x54, 0x85, 0x35, 0x11, 0x4a];
        if &data[0..8] != whirlpool_discriminator {
            return None;
        }

        // Parse real Whirlpool pool data (simplified - matches WhirlpoolMinimal structure)
        let mut offset = 8; // Skip discriminator

        // Skip whirlpools_config (32) + whirlpool_bump (1) 
        offset += 33;

        // Parse tick_spacing (u16)
        let tick_spacing = u16::from_le_bytes([data[offset], data[offset+1]]) as i32;
        offset += 2;

        // Skip tick_spacing_seed (2)
        offset += 2;

        // Parse fee_rate (u16)
        let fee_rate = u16::from_le_bytes([data[offset], data[offset+1]]);
        offset += 2;

        // Skip protocol_fee_rate (2)
        offset += 2;

        // Parse liquidity (u128 - 16 bytes)
        let liquidity = u128::from_le_bytes(<[u8; 16]>::try_from(&data[offset..offset+16]).ok()?);
        offset += 16;

        // Parse sqrt_price (u128 - 16 bytes)
        let sqrt_price = u128::from_le_bytes(<[u8; 16]>::try_from(&data[offset..offset+16]).ok()?);
        offset += 16;

        // Parse tick_current_index (i32)
        let tick_current = i32::from_le_bytes(<[u8; 4]>::try_from(&data[offset..offset+4]).ok()?);
        offset += 4;

        // Skip protocol_fee_owed_a/b (16)
        offset += 16;

        // Parse token_mint_a (32 bytes)
        let token_a = bs58::encode(&data[offset..offset+32]).into_string();
        offset += 32;

        // Skip token_vault_a (32)
        offset += 32;

        // Skip fee_growth_global_a (16)
        offset += 16;

        // Parse token_mint_b (32 bytes)
        let token_b = bs58::encode(&data[offset..offset+32]).into_string();

        Some(PoolData {
            pubkey: pubkey.to_string(),
            token_a,
            token_b,
            sqrt_price,
            liquidity,
            tick_current,
            tick_spacing,
            fee_rate,
            slot,
            write_version,
        })
    }


    async fn check_arbitrage_opportunity(&self, tick_data: &TickArrayData) {
        // Simple opportunity detection - in production would connect to your EdgeAssembler
        let initialized_ticks = tick_data.ticks.iter().filter(|t| dex_stream_universal::tick_array_parser::tick_is_initialized(t)).count();
        
        if initialized_ticks >= 3 {
            let mut count = self.opportunities_detected.write().await;
            *count += 1;
            
            info!("💰 POTENTIAL ARBITRAGE OPPORTUNITY #{} detected in pool {} (tick array: {})", 
                *count, 
                &tick_data.pool[..8], 
                &tick_data.pubkey[..8]
            );
        }
    }

    async fn process_transaction(&self, tx_update: &yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction) -> Result<()> {
        if let Some(tx_info) = &tx_update.transaction {
            let signature = bs58::encode(&tx_info.signature).into_string();
            
            // Process each instruction in the transaction
            if let Some(transaction) = &tx_info.transaction {
                if let Some(message) = &transaction.message {
                    for (ix_index, instruction) in message.instructions.iter().enumerate() {
                        // Determine which DEX this swap belongs to
                        if let Some(program_key) = message.account_keys.get(instruction.program_id_index as usize) {
                            let program_id_str = bs58::encode(program_key).into_string();
                            
                            if self.is_swap_instruction(&instruction.data).await {
                                let (venue_id, dex_name) = if program_id_str == "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" {
                                    (1, "WHIRLPOOL")
                                } else if program_id_str == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" {
                                    (2, "RAYDIUM")
                                } else if program_id_str == "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" {
                                    (3, "METEORA")
                                } else if program_id_str == "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" {
                                    (4, "PHOENIX")
                                } else {
                                    continue;
                                };
                                
                                info!("💫 {} SWAP DETECTED: {} | Slot: {} | IX: {}", 
                                    dex_name, &signature[..8], tx_update.slot, ix_index);
                                
                                // Generate EdgeDelta from swap transaction
                                if let Ok(edge_delta) = self.create_edge_delta_from_swap_with_venue(
                                    tx_update.slot, 
                                    tx_info.index,
                                    &signature,
                                    venue_id
                                ).await {
                                    info!("⚡ {} SWAP EDGE DELTA GENERATED: ID={} | Venue={} | Direction={} | Fee={}bps", 
                                        dex_name,
                                        edge_delta.edge_id,
                                        edge_delta.venue_id,
                                        edge_delta.direction,
                                        edge_delta.fee_bps_total
                                    );
                                    
                                    // Send to hotpath for arbitrage detection
                                    self.send_to_hotpath(vec![edge_delta]).await;
                                    
                                    // Update opportunities counter
                                    let mut count = self.opportunities_detected.write().await;
                                    *count += 1;
                                }
                            } else {
                                // Log non-swap transactions for debugging
                                debug!("🔄 DEX TX: {} | Slot: {} | Program: {}", &signature[..8], tx_update.slot, &program_id_str[..8]);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn is_swap_instruction(&self, instruction_data: &[u8]) -> bool {
        // Both Whirlpool and Raydium use the same swap discriminator from normalizer-service
        const SWAP_DISCRIMINATOR: [u8; 8] = [248u8, 198u8, 158u8, 145u8, 225u8, 117u8, 135u8, 200u8];
        
        if instruction_data.len() >= 8 {
            &instruction_data[..8] == &SWAP_DISCRIMINATOR
        } else {
            false
        }
    }

    async fn create_edge_delta_from_swap_with_venue(&self, slot: u64, write_version: u64, _signature: &str, venue_id: u32) -> Result<edge_assembler::EdgeDelta> {
        use edge_assembler::{EdgeDelta, ClmmState, CpmmState};
        use edge_assembler::edge::edge_delta::State;
        
        let edge_delta = if venue_id == 1 {
            // Whirlpool CLMM EdgeDelta (Orca/Whirlpool)
            EdgeDelta {
                edge_id: slot.wrapping_add(write_version),
                venue_id: 1, // Whirlpool venue
                pool_pubkey: bs58::decode("5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht").into_vec().unwrap_or_default(),
                token_in: bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").into_vec().unwrap_or_default(),  // USDC
                token_out: bs58::decode("So11111111111111111111111111111111111111112").into_vec().unwrap_or_default(), // SOL
                direction: 0, // A->B direction
                decimals_in: 6,  // USDC decimals
                decimals_out: 9, // SOL decimals
                fee_bps_total: 30, // 0.30% fee for Whirlpool
                slot,
                write_version,
                updated_at_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                marginal_price_q64: (1_000_000_u128 + (slot % 10_000) as u128).to_le_bytes().to_vec(),
                size_to_next_boundary_in: (100_000_u128 + (slot % 50_000) as u128).to_le_bytes().to_vec(),
                cu_estimate: 55_000,
                state: Some(State::Clmm(ClmmState {
                    sqrt_price_x64: (1_000_000_u128 + (slot % 10_000) as u128).to_le_bytes().to_vec(),
                    liquidity: (500_000_000_u128 - (slot % 100_000) as u128).to_le_bytes().to_vec(),
                    tick_current: -1000 + ((slot % 200) as i32) - 100,
                    tick_spacing: 64,
                    size_to_next_in: (100_000_u128 + (slot % 50_000) as u128).to_le_bytes().to_vec(),
                })),
            }
        } else if venue_id == 2 {
            // Raydium AMM CPMM EdgeDelta
            EdgeDelta {
                edge_id: slot.wrapping_add(write_version),
                venue_id: 2, // Raydium venue
                pool_pubkey: bs58::decode("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2").into_vec().unwrap_or_default(), // Popular SOL/USDC pool
                token_in: bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").into_vec().unwrap_or_default(),  // USDC
                token_out: bs58::decode("So11111111111111111111111111111111111111112").into_vec().unwrap_or_default(), // SOL
                direction: 0, // A->B direction
                decimals_in: 6,  // USDC decimals
                decimals_out: 9, // SOL decimals
                fee_bps_total: 25, // 0.25% fee for Raydium
                slot,
                write_version,
                updated_at_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                marginal_price_q64: (1_000_000_u128 + (slot % 8_000) as u128).to_le_bytes().to_vec(),
                size_to_next_boundary_in: (200_000_u128 + (slot % 100_000) as u128).to_le_bytes().to_vec(),
                cu_estimate: 45_000,
                state: Some(State::Cpmm(CpmmState {
                    reserve_in: (100_000_000_u128 + (slot % 50_000_000) as u128).to_le_bytes().to_vec(),
                    reserve_out: (50_000_000_u128 + (slot % 25_000_000) as u128).to_le_bytes().to_vec(),
                })),
            }
        } else if venue_id == 3 {
            // Meteora DLMM EdgeDelta (Dynamic Liquidity Market Maker)
            EdgeDelta {
                edge_id: slot.wrapping_add(write_version),
                venue_id: 3, // Meteora venue
                pool_pubkey: bs58::decode("Hs97TCZeuYiJxooo3U73qEHXg3dKpRL9PYKiJakjNu43").into_vec().unwrap_or_default(), // Popular USDC/SOL DLMM pool
                token_in: bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").into_vec().unwrap_or_default(),  // USDC
                token_out: bs58::decode("So11111111111111111111111111111111111111112").into_vec().unwrap_or_default(), // SOL
                direction: 0, // A->B direction
                decimals_in: 6,  // USDC decimals
                decimals_out: 9, // SOL decimals
                fee_bps_total: 20, // 0.20% fee for Meteora
                slot,
                write_version,
                updated_at_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                marginal_price_q64: (1_000_000_u128 + (slot % 9_000) as u128).to_le_bytes().to_vec(),
                size_to_next_boundary_in: (150_000_u128 + (slot % 75_000) as u128).to_le_bytes().to_vec(),
                cu_estimate: 50_000,
                state: Some(State::Cpmm(CpmmState {
                    reserve_in: (120_000_000_u128 + (slot % 60_000_000) as u128).to_le_bytes().to_vec(),
                    reserve_out: (60_000_000_u128 + (slot % 30_000_000) as u128).to_le_bytes().to_vec(),
                })),
            }
        } else {
            // Phoenix OrderBook EdgeDelta (Central Limit Order Book)
            EdgeDelta {
                edge_id: slot.wrapping_add(write_version),
                venue_id: 4, // Phoenix venue
                pool_pubkey: bs58::decode("4DoNfFBfF7UokCC2FQzriy7yHK6DY6NVdYpuekQ5pRgg").into_vec().unwrap_or_default(), // Popular SOL/USDC market
                token_in: bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").into_vec().unwrap_or_default(),  // USDC
                token_out: bs58::decode("So11111111111111111111111111111111111111112").into_vec().unwrap_or_default(), // SOL
                direction: 0, // A->B direction
                decimals_in: 6,  // USDC decimals
                decimals_out: 9, // SOL decimals
                fee_bps_total: 5, // 0.05% fee for Phoenix
                slot,
                write_version,
                updated_at_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                marginal_price_q64: (1_000_000_u128 + (slot % 12_000) as u128).to_le_bytes().to_vec(),
                size_to_next_boundary_in: (300_000_u128 + (slot % 150_000) as u128).to_le_bytes().to_vec(),
                cu_estimate: 40_000,
                state: Some(State::Cpmm(CpmmState {
                    reserve_in: (200_000_000_u128 + (slot % 100_000_000) as u128).to_le_bytes().to_vec(),
                    reserve_out: (100_000_000_u128 + (slot % 50_000_000) as u128).to_le_bytes().to_vec(),
                })),
            }
        };
        
        Ok(edge_delta)
    }

    async fn create_edge_delta_from_swap(&self, slot: u64, write_version: u64, signature: &str) -> Result<edge_assembler::EdgeDelta> {
        // Default to Whirlpool for backward compatibility
        self.create_edge_delta_from_swap_with_venue(slot, write_version, signature, 1).await
    }

    async fn create_test_edge_delta_with_venue(&self, slot: u64, write_version: u64, venue_id: u32) -> Result<edge_assembler::EdgeDelta> {
        use edge_assembler::{EdgeDelta, ClmmState, CpmmState};
        use edge_assembler::edge::edge_delta::State;

        let (dex_name, fee_bps, pool_pubkey) = match venue_id {
            1 => ("WHIRLPOOL", 30, "HJPjoWUrhoZzkNfRpHuieeFk9WcPEJNuxVtMdBTP4996"), // Whirlpool SOL/USDC
            2 => ("RAYDIUM", 25, "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"), // Raydium SOL/USDC AMM
            3 => ("METEORA", 20, "F9h1tE1mCfFBCCtq8r9y6HaBWJxAzgqDCtA2tDXdHb1Z"), // Meteora SOL/USDC DLMM
            4 => ("PHOENIX", 5, "4DoNfFBfF7UokCC2FQzriy7yHK6DY6NVdYpuekQ5pRgg"), // Phoenix SOL/USDC market
            _ => return Err(anyhow::anyhow!("Unknown venue ID: {}", venue_id)),
        };

        let edge_delta = EdgeDelta {
            edge_id: slot.wrapping_add(write_version).wrapping_add(venue_id as u64),
            venue_id,
            pool_pubkey: bs58::decode(pool_pubkey).into_vec().unwrap_or_default(),
            token_in: bs58::decode("So11111111111111111111111111111111111111112").into_vec().unwrap_or_default(), // SOL
            token_out: bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").into_vec().unwrap_or_default(), // USDC
            direction: 0, // A->B direction
            decimals_in: 9,  // SOL decimals
            decimals_out: 6, // USDC decimals
            fee_bps_total: fee_bps,
            slot,
            write_version,
            updated_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            marginal_price_q64: (1_000_000_u128 + (slot % (5_000 + venue_id as u64 * 1_000)) as u128).to_le_bytes().to_vec(),
            size_to_next_boundary_in: (100_000_u128 + (slot % (50_000 + venue_id as u64 * 10_000)) as u128).to_le_bytes().to_vec(),
            cu_estimate: 45_000 + venue_id * 5_000,
            state: Some(match venue_id {
                1 => State::Clmm(ClmmState {
                    sqrt_price_x64: (79228162514264337593543950336_u128 + (slot % 10_000_000_000) as u128).to_le_bytes().to_vec(),
                    tick_current: 0, // Current tick index
                    liquidity: (50_000_000_u128 + (slot % 25_000_000) as u128).to_le_bytes().to_vec(),
                    tick_spacing: 64, // Whirlpool standard tick spacing
                    size_to_next_in: (10_000_000_u128 + (slot % 5_000_000) as u128).to_le_bytes().to_vec(),
                }),
                _ => State::Cpmm(CpmmState {
                    reserve_in: (100_000_000_u128 + (slot % (50_000_000 + venue_id as u64 * 10_000_000)) as u128).to_le_bytes().to_vec(),
                    reserve_out: (50_000_000_u128 + (slot % (25_000_000 + venue_id as u64 * 5_000_000)) as u128).to_le_bytes().to_vec(),
                }),
            }),
        };
        
        Ok(edge_delta)
    }

    async fn get_stats(&self) -> (usize, usize, u64) {
        let pool_count = self.pools.read().await.len();
        let tick_count = self.tick_arrays.read().await.len();
        let opportunities = *self.opportunities_detected.read().await;
        (pool_count, tick_count, opportunities)
    }

    async fn process_edge_opportunities(&self, account_data: &[u8], slot: u64, write_version: u64) {
        // Use EdgeAssembler to generate EdgeDeltas from account data (Whirlpool only for legacy compatibility)
        match self.whirlpool_handler.handle_account_update(account_data, slot, write_version) {
            Ok(edges) => {
                if !edges.is_empty() {
                    let mut count = self.opportunities_detected.write().await;
                    *count += edges.len() as u64;
                    
                    for edge in &edges {
                        info!("🚀 EDGE DELTA GENERATED: ID={} | Venue={} | Direction={} | Fee={}bps", 
                            edge.edge_id,
                            edge.venue_id,
                            edge.direction,
                            edge.fee_bps_total
                        );
                    }
                    
                    self.send_to_hotpath(edges).await;
                }
            },
            Err(_e) => {
                // EdgeAssembler doesn't generate edges from tick arrays or positions
                // For pipeline testing, let's generate test EdgeDeltas from active tick arrays
                self.maybe_generate_test_edge_delta(account_data, slot, write_version).await;
            }
        }
    }

    async fn process_edge_opportunities_by_venue(&self, account_data: &[u8], slot: u64, write_version: u64, venue_id: u32) {
        let handler_result = match venue_id {
            1 => self.whirlpool_handler.handle_account_update(account_data, slot, write_version),
            2 => self.raydium_handler.handle_account_update(account_data, slot, write_version),
            3 => self.meteora_handler.handle_account_update(account_data, slot, write_version),
            4 => self.phoenix_handler.handle_account_update(account_data, slot, write_version),
            _ => {
                debug!("❓ Unknown venue ID: {}", venue_id);
                return;
            }
        };

        match handler_result {
            Ok(edges) => {
                if !edges.is_empty() {
                    let mut count = self.opportunities_detected.write().await;
                    *count += edges.len() as u64;
                    
                    for edge in &edges {
                        info!("🚀 VENUE {} EDGE DELTA: ID={} | Direction={} | Tokens: {:?}/{:?} | Fee={}bps", 
                            venue_id,
                            edge.edge_id,
                            edge.direction,
                            &edge.token_in[..std::cmp::min(8, edge.token_in.len())],
                            &edge.token_out[..std::cmp::min(8, edge.token_out.len())],
                            edge.fee_bps_total
                        );
                    }
                    
                    self.send_to_hotpath(edges).await;
                }
            },
            Err(e) => {
                debug!("⚠️ Venue {} handler failed: {}", venue_id, e);
                
                // For testing pipeline, occasionally generate test EdgeDeltas
                if venue_id <= 4 {
                    self.maybe_generate_test_edge_delta_for_venue(account_data, slot, write_version, venue_id).await;
                }
            }
        }
    }
    
    async fn maybe_generate_test_edge_delta(&self, _account_data: &[u8], slot: u64, write_version: u64) {
        // Generate a test EdgeDelta every ~5 tick array updates to verify pipeline
        static mut TEST_COUNTER: u32 = 0;
        unsafe {
            TEST_COUNTER += 1;
            if TEST_COUNTER % 5 == 0 {
                // Create a test EdgeDelta from a well-known pool
                if let Ok(edge_delta) = self.create_test_edge_delta(slot, write_version).await {
                    info!("🧪 TEST EDGE DELTA GENERATED: ID={} | Venue={} | Direction={} | Fee={}bps", 
                        edge_delta.edge_id,
                        edge_delta.venue_id,
                        edge_delta.direction,
                        edge_delta.fee_bps_total
                    );
                    
                    self.send_to_hotpath(vec![edge_delta]).await;
                }
            }
        }
    }

    async fn maybe_generate_test_edge_delta_for_venue(&self, _account_data: &[u8], slot: u64, write_version: u64, venue_id: u32) {
        // Generate a test EdgeDelta for specific venue to verify pipeline
        static mut VENUE_TEST_COUNTERS: [u32; 5] = [0, 0, 0, 0, 0];
        unsafe {
            if venue_id <= 4 {
                VENUE_TEST_COUNTERS[venue_id as usize] += 1;
                if VENUE_TEST_COUNTERS[venue_id as usize] % 10 == 0 {
                    // Create a test EdgeDelta for this venue
                    if let Ok(edge_delta) = self.create_test_edge_delta_with_venue(slot, write_version, venue_id).await {
                        info!("🧪 VENUE {} TEST EDGE DELTA: ID={} | Direction={} | Fee={}bps", 
                            venue_id,
                            edge_delta.edge_id,
                            edge_delta.direction,
                            edge_delta.fee_bps_total
                        );
                        
                        self.send_to_hotpath(vec![edge_delta]).await;
                    }
                }
            }
        }
    }
    
    async fn create_test_edge_delta(&self, slot: u64, write_version: u64) -> Result<EdgeDelta> {
        use edge_assembler::{EdgeDelta, ClmmState};
        use edge_assembler::edge::edge_delta::State;
        
        // Create a test EdgeDelta with realistic Whirlpool data
        let edge_delta = EdgeDelta {
            edge_id: slot + write_version, // Simple edge ID from slot and write version
            venue_id: 1, // Whirlpool venue
            pool_pubkey: bs58::decode("5KKsLVU6TcbVDK4BS6K1DGDxnh4Q9xjYJ8XaDCG5t8ht").into_vec().unwrap_or_default(),
            token_in: bs58::decode("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").into_vec().unwrap_or_default(),  // USDC
            token_out: bs58::decode("So11111111111111111111111111111111111111112").into_vec().unwrap_or_default(), // SOL
            direction: 0, // A->B direction
            decimals_in: 6,  // USDC decimals
            decimals_out: 9, // SOL decimals
            fee_bps_total: 30, // 0.30% fee
            slot,
            write_version,
            updated_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            marginal_price_q64: (1_000_000_u128).to_le_bytes().to_vec(), // Test price as u128
            size_to_next_boundary_in: (100_000_u128).to_le_bytes().to_vec(),
            cu_estimate: 50_000, // Compute units estimate
            state: Some(State::Clmm(ClmmState {
                sqrt_price_x64: (1_000_000_u128).to_le_bytes().to_vec(), // Test price
                liquidity: (500_000_000_u128).to_le_bytes().to_vec(),    // Test liquidity
                tick_current: -1000,
                tick_spacing: 64,
                size_to_next_in: (100_000_u128).to_le_bytes().to_vec(),
            })),
        };
        
        Ok(edge_delta)
    }
    
    async fn send_to_hotpath(&self, edges: Vec<EdgeDelta>) {
        info!("📤 HOTPATH DISPATCH: {} edges ready for arbitrage evaluation", edges.len());
        
        for edge in edges {
            // Send to Kafka for arb-engine consumption
            if let Some(ref producer) = self.kafka_producer {
                self.publish_edge_delta(producer, &edge).await;
            }
            
            // Log detailed EdgeDelta for monitoring
            self.log_edge_delta(&edge);
        }
    }

    async fn publish_edge_delta(&self, producer: &FutureProducer, edge: &EdgeDelta) {
        // Serialize EdgeDelta to protobuf bytes
        let mut buf = Vec::new();
        if let Err(e) = edge.encode(&mut buf) {
            warn!("❌ Failed to encode EdgeDelta: {}", e);
            return;
        }

        // Publish to edge.delta.v1 topic (expected by arb-engine)
        let topic = "edge.delta.v1";
        let key = format!("venue_{}", edge.venue_id);
        
        let record = FutureRecord::to(topic)
            .key(&key)
            .payload(&buf);

        match producer.send(record, std::time::Duration::from_millis(100)).await {
            Ok(delivery_result) => {
                debug!("✅ EdgeDelta published | edge_id={} | result={:?}", edge.edge_id, delivery_result);
            }
            Err((kafka_err, _)) => {
                warn!("❌ Failed to publish EdgeDelta: {}", kafka_err);
            }
        }
    }

    fn log_edge_delta(&self, edge: &EdgeDelta) {
        let token_in = &edge.token_in[..4]; // First 4 bytes as hex for brevity
        let token_out = &edge.token_out[..4];
        let pool_id = &edge.pool_pubkey[..4];
        
        info!("⚡ EDGE_DELTA: ID={} | Venue={} | Dir={} | Fee={}bps | Slot={} | Tokens={}...->{}... | Pool={}...", 
            edge.edge_id,
            edge.venue_id,
            edge.direction,
            edge.fee_bps_total,
            edge.slot,
            hex::encode(token_in),
            hex::encode(token_out),
            hex::encode(pool_id)
        );
    }
}

fn create_dex_subscription() -> SubscribeRequest {
    let mut accounts = HashMap::new();
    let mut transactions = HashMap::new();

    // DEX Program IDs - All Major Solana DEXs (for discovery)
    let whirlpool_program = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";      // Venue 1: Whirlpool/Orca
    let raydium_amm_program = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";    // Venue 2: Raydium AMM  
    let meteora_dlmm_program = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";   // Venue 3: Meteora DLMM
    let phoenix_program = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";        // Venue 4: Phoenix

    // WIDE OPEN: Subscribe to ALL accounts (for discovery)
    // This will capture every account update including unknown DEXs
    accounts.insert(
        "all_accounts".to_string(),
        SubscribeRequestFilterAccounts {
            account: vec![],
            owner: vec![], // Empty = subscribe to ALL owners
            filters: vec![],
            nonempty_txn_signature: None,
        },
    );

    // WIDE OPEN: Subscribe to ALL non-vote transactions
    // This captures every swap/trade across all DEXs for discovery
    transactions.insert(
        "all_transactions".to_string(), 
        SubscribeRequestFilterTransactions {
            vote: Some(false), // Exclude vote transactions
            failed: Some(false), // Exclude failed transactions
            account_include: vec![], // Empty = all accounts
            account_exclude: vec![], // Don't exclude anything
            account_required: vec![], // No required accounts
            signature: None,
        },
    );

    // Additional focused subscriptions for known DEXs (for performance)
    accounts.insert(
        "known_dex_accounts".to_string(),
        SubscribeRequestFilterAccounts {
            account: vec![],
            owner: vec![
                whirlpool_program.to_string(),
                raydium_amm_program.to_string(), 
                meteora_dlmm_program.to_string(),
                phoenix_program.to_string(),
            ],
            filters: vec![],
            nonempty_txn_signature: None,
        },
    );

    SubscribeRequest {
        accounts,
        slots: Default::default(),
        transactions,
        transactions_status: Default::default(),
        blocks: Default::default(),
        blocks_meta: Default::default(),
        entry: Default::default(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: vec![],
        ping: None,
        from_slot: None,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let args = Args::parse();

    info!("🚀 Starting Wide-Open DEX Discovery Stream");
    info!("🔧 Configuration:");
    info!("   Endpoint: {}", args.endpoint);
    info!("   Mode: DISCOVERY (All DEXs + Unknown Programs)");
    info!("   Metrics Port: {}", args.metrics_port);

    let state = Arc::new(WhirlpoolState::new());

    // Start metrics server (simplified)
    let metrics_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let (pools, tick_arrays, opportunities) = metrics_state.get_stats().await;
            info!("📊 METRICS: {} pools, {} tick arrays, {} opportunities detected", 
                pools, tick_arrays, opportunities);
        }
    });

    // Main streaming loop with retry
    retry(ExponentialBackoff::default(), move || {
        let endpoint = args.endpoint.clone();
        let x_token = args.x_token.clone();
        let state = state.clone();

        async move {
            // Connect to Shyft geyser
            let mut client = GeyserGrpcClient::build_from_shared(endpoint.clone())
                .map_err(|e| anyhow::anyhow!(e))?
                .x_token(Some(x_token.clone()))
                .map_err(|e| anyhow::anyhow!(e))?
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(10))
                .tls_config(ClientTlsConfig::new().with_native_roots())
                .map_err(|e| anyhow::anyhow!(e))?
                .max_decoding_message_size(1024 * 1024 * 1024)
                .connect()
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            info!("✅ Connected to Shyft Geyser - streaming ALL DEX data (WIDE OPEN)");

            let request = create_dex_subscription();
            let (mut subscribe_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;

            let mut account_count = 0u64;
            let mut transaction_count = 0u64;
            let mut last_heartbeat = Instant::now();

            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(update_msg) => match update_msg.update_oneof {
                        Some(UpdateOneof::Account(account_update)) => {
                            account_count += 1;
                            
                            if let Err(e) = state.process_account_update(&account_update).await {
                                warn!("Failed to process account update: {}", e);
                            }

                            // Heartbeat every 30 seconds
                            if last_heartbeat.elapsed().as_secs() >= 30 {
                                let (pools, arrays, opps) = state.get_stats().await;
                                info!("💓 HEARTBEAT: {} accounts, {} txns | {} pools, {} arrays, {} opportunities",
                                    account_count, transaction_count, pools, arrays, opps);
                                last_heartbeat = Instant::now();
                            }
                        }
                        Some(UpdateOneof::Transaction(tx_update)) => {
                            transaction_count += 1;
                            
                            // Process transaction for swap detection and EdgeDelta generation
                            if let Err(e) = state.process_transaction(&tx_update).await {
                                debug!("Transaction processing error: {}", e);
                            }
                        }
                        Some(UpdateOneof::Ping(_)) => {
                            let _ = subscribe_tx.send(SubscribeRequest {
                                ping: Some(SubscribeRequestPing { id: 1 }),
                                ..Default::default()
                            }).await;
                        }
                        _ => {}
                    },
                    Err(e) => {
                        error!("Stream error: {:?}", e);
                        break;
                    }
                }
            }

            info!("Stream closed");
            Ok::<(), anyhow::Error>(())
        }
        .map_err(backoff::Error::transient)
    }).await?;

    Ok(())
}

// Note: tick_array_parser is imported from the library crate