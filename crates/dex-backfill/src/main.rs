use chrono::Utc;
use clap::{Parser, ArgAction};
use dotenv::dotenv;
use env_logger;
use log::{error, info, warn};
use redis::{aio::MultiplexedConnection, AsyncCommands, pipe};
use std::{collections::HashMap, env, time::Duration};
use tokio::sync::Mutex;
use std::sync::Arc;
use bs58;
use lru::LruCache;

use whirlpool_interface::accounts::{
    Whirlpool, WhirlpoolAccount, WHIRLPOOL_ACCOUNT_DISCM,
    TICK_ARRAY_ACCOUNT_DISCM, POSITION_ACCOUNT_DISCM,
};
use whirlpool_interface::accounts::TickArrayAccount;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::geyser::{
    subscribe_request_filter_accounts_filter::Filter as AccountsFilterOneOf,
    subscribe_request_filter_accounts_filter_memcmp::Data as MemcmpData,
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
    SubscribeRequestFilterAccountsFilter, SubscribeRequestFilterAccountsFilterMemcmp,
    subscribe_update::UpdateOneof, SubscribeUpdateAccount,
};
use futures_util::StreamExt;

type AccountFilterMap = HashMap<String, SubscribeRequestFilterAccounts>;

const WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";


#[derive(Clone, Debug, Default)]
struct MintMeta {
    decimals: u8,
    symbol: Option<String>,
    name: Option<String>,
}

struct MintCache {
    lru: LruCache<String, MintMeta>,
}

impl MintCache {
    fn new(cap: usize) -> Self {
        use std::num::NonZeroUsize;
        Self { lru: LruCache::new(NonZeroUsize::new(cap.max(16)).unwrap()) }
    }

    /// Pull from in-memory LRU first; if missing, read from Redis: HGET dex:mint:{MINT} decimals/symbol/name
    async fn get_from_redis(
        &mut self,
        redis: &mut redis::aio::MultiplexedConnection,
        mint: &str,
    ) -> anyhow::Result<MintMeta> {
        if let Some(meta) = self.lru.get(mint).cloned() {
            return Ok(meta);
        }
        let key = format!("dex:mint:{mint}");

        // HMGET returns a tuple of Options; missing fields are OK.
        // (We keep it tolerant; your backfill and streamer will fill these over time.)
        let (dec_opt, sym_opt, name_opt): (Option<i64>, Option<String>, Option<String>) =
            redis::cmd("HMGET")
                .arg(&key).arg("decimals").arg("symbol").arg("name")
                .query_async(redis).await
                .unwrap_or((None, None, None));

        let meta = MintMeta {
            decimals: dec_opt.map(|d| d as u8).unwrap_or(0), // 0 if unknown (you can default to 6 if you prefer)
            symbol: sym_opt,
            name: name_opt,
        };

        self.lru.put(mint.to_string(), meta.clone());
        Ok(meta)
    }
}



#[derive(Debug, Clone)]
struct ConnArgs {
    endpoint: String,
    x_token: String,
    redis_url: String,
}

impl ConnArgs {
    async fn connect(&self) -> anyhow::Result<GeyserGrpcClient<impl Interceptor>> {
        info!("🔒 Building gRPC client for endpoint: {}", self.endpoint);
        
        let builder = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .x_token(Some(self.x_token.clone()))?
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(20))
            .tls_config(tonic::transport::channel::ClientTlsConfig::new().with_native_roots())?
            .max_decoding_message_size(1024 * 1024 * 1024);
        
        info!("🚀 Attempting connection...");
        builder.connect().await.map_err(Into::into)
    }
}

#[derive(Debug, Clone, Parser)]
#[clap(author, version, about)]
struct AppArgs {
    /// Subscribe to ALL accounts owned by the Whirlpool program (debug only)
    #[arg(long, env = "OWNER_ONLY", action = ArgAction::SetTrue)]
    owner_only: bool,

    /// Include TickArray accounts (stored to a Redis stream)
    #[arg(long, env = "INCLUDE_TICKARRAYS", action = ArgAction::SetTrue)]
    include_tickarrays: bool,

    /// Include Position accounts (stored to a Redis stream)
    #[arg(long, env = "INCLUDE_POSITIONS", action = ArgAction::SetTrue)]
    include_positions: bool,

    /// Backfill starting slot (optional)
    #[arg(long, env = "FROM_SLOT")]
    from_slot: Option<u64>,
}

impl AppArgs {
    fn build_connection_args(&self) -> ConnArgs {
        let endpoint = env::var("ENDPOINT").expect("ENDPOINT not found in .env file");
        let x_token = env::var("X_TOKEN").expect("X_TOKEN not found in .env file");
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL not found in .env file");
        
        ConnArgs { endpoint, x_token, redis_url }
    }

    fn build_subscribe_request(&self) -> SubscribeRequest {
        let mut accounts: AccountFilterMap = HashMap::new();

        // helper: owner + optional memcmp(0, disc)
        let mut push = |label: &str, owner: &str, disc: Option<[u8; 8]>| {
            let filters = match disc {
                Some(bytes8) => vec![SubscribeRequestFilterAccountsFilter {
                    filter: Some(AccountsFilterOneOf::Memcmp(
                        SubscribeRequestFilterAccountsFilterMemcmp {
                            offset: 0,
                            data: Some(MemcmpData::Bytes(bytes8.to_vec())),
                        },
                    )),
                }],
                None => vec![],
            };
            accounts.insert(
                label.to_string(),
                SubscribeRequestFilterAccounts {
                    account: vec![],
                    owner: vec![owner.to_string()],
                    nonempty_txn_signature: None,
                    filters,
                },
            );
        };

        if self.owner_only {
            // Firehose
            push("whirlpool_owner_only", WHIRLPOOL_PROGRAM_ID, None);
        } else {
            // Safe default: only Whirlpool POOL accounts (ok to IDL-parse)
            push("whirlpool_pools", WHIRLPOOL_PROGRAM_ID, Some(WHIRLPOOL_ACCOUNT_DISCM));
            if self.include_tickarrays {
                push("whirlpool_tickarrays", WHIRLPOOL_PROGRAM_ID, Some(TICK_ARRAY_ACCOUNT_DISCM));
            }
            if self.include_positions {
                push("whirlpool_positions", WHIRLPOOL_PROGRAM_ID, Some(POSITION_ACCOUNT_DISCM));
            }
        }

        SubscribeRequest {
            accounts,
            slots: HashMap::default(),
            transactions: HashMap::default(),
            transactions_status: HashMap::default(),
            blocks: HashMap::default(),
            blocks_meta: HashMap::default(),
            entry: HashMap::default(),
            commitment: Some(CommitmentLevel::Processed as i32),
            accounts_data_slice: Vec::default(),
            ping: None,
            from_slot: self.from_slot,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    // Set up logging
    env::set_var(
        env_logger::DEFAULT_FILTER_ENV,
        env::var_os(env_logger::DEFAULT_FILTER_ENV).unwrap_or_else(|| "info".into()),
    );
    env_logger::init();

    // At the start of your main function
    info!("🔍 Loading IDL files...");
    info!("📁 Current working directory: {:?}", std::env::current_dir()?);

    // Check if IDL files exist
            let idl_path = "idls/whirLbMi_idl.json";
    if std::path::Path::new(idl_path).exists() {
        info!("✅ IDL file found: {}", idl_path);
    } else {
        error!("❌ IDL file not found: {}", idl_path);
    }

    // Read environment variables
    let endpoint = env::var("ENDPOINT").expect("ENDPOINT not found in .env file");
    let x_token = env::var("X_TOKEN").expect("X_TOKEN not found in .env file");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL not found in .env file");
    
    // Filter flags
    let flags = AppArgs::parse();

    let _zero_attempts = Arc::new(Mutex::new(true));

    // Display configuration at startup
    println!("🚀 Starting Whirlpool Account Updates Streamer");
    println!("================================================");
    println!("📡 Endpoint: {}", endpoint);
    println!("🔑 X-Token: {}...{}", &x_token[..8], &x_token[x_token.len()-4..]);
    println!("🗄️  Redis URL: {}", redis_url);
    println!("⚙️  Configuration:");
    println!("   Owner only mode: {}", flags.owner_only);
    println!("   Include tick arrays: {}", flags.include_tickarrays);
    println!("   Include positions: {}", flags.include_positions);
    if let Some(from_slot) = flags.from_slot {
        println!("   Starting from slot: {}", from_slot);
    } else {
        println!("   Starting from latest slot");
    }
    println!("================================================");

    info!("Starting Whirlpool streamer");
    info!("Endpoint: {}", endpoint);
    info!("Redis: {}", redis_url);
    info!("X-Token: {}", x_token);
    info!("Configuration flags: {:?}", flags);

    // Network creds
    let _conn_args = ConnArgs { endpoint, x_token, redis_url };

    // Simple retry loop for connection
    let mut attempts = 0;
    let max_attempts = 5;
    
    loop {
        attempts += 1;
        info!("🔄 Attempt {} of {}: Attempting to connect to geyser...", attempts, max_attempts);
        
        match try_connect(&flags).await {
            Ok(_) => {
                info!("✅ Connection and processing completed successfully");
                break;
            }
            Err(e) => {
                error!("❌ Attempt {} failed: {}", attempts, e);
                if attempts >= max_attempts {
                    error!("❌ Failed to establish connection after {} attempts", max_attempts);
                    return Err(e);
                }
                
                // Exponential backoff: wait longer between attempts
                let delay = std::cmp::min(1000 * (2_u64.pow(attempts as u32)), 30000);
                info!("⏳ Waiting {}ms before retry...", delay);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
        }
    }

    Ok(())
}

async fn try_connect(flags: &AppArgs) -> anyhow::Result<()> {
    info!("🔌 Building connection arguments...");
    let conn_args = flags.build_connection_args();
    
    info!("📡 Connection details:");
    info!("   Endpoint: {}", conn_args.endpoint);
    info!("   X-Token: {}...{}", 
        &conn_args.x_token[..8], 
        &conn_args.x_token[conn_args.x_token.len()-4..]
    );
    info!("   Redis URL: {}", conn_args.redis_url);
    
    info!("🔄 Attempting to connect to geyser...");
    info!("   Setting connection timeout: 20s");
    info!("   Setting request timeout: 20s");
    info!("   Max message size: 1GB");
    
    let client = match conn_args.connect().await {
        Ok(client) => {
            info!("✅ Successfully connected to geyser!");
            client
        }
        Err(e) => {
            error!("❌ Failed to connect to geyser");
            error!("🔍 Connection error details: {:#?}", e);
            error!("📡 Endpoint: {}", conn_args.endpoint);
            error!("🔑 X-Token: {}...{}", 
                &conn_args.x_token[..8], 
                &conn_args.x_token[conn_args.x_token.len()-4..]
            );
            
            // Check if it's a TLS/certificate issue
            if e.to_string().contains("certificate") || e.to_string().contains("UnknownIssuer") {
                error!("🔒 TLS Certificate Issue Detected");
                error!("💡 This might be a certificate validation problem with the endpoint");
                error!("💡 Try checking if the endpoint URL is correct or if there are certificate issues");
            }
            
            // Check if it's a network issue
            if e.to_string().contains("transport") {
                error!("🌐 Network/Transport Issue Detected");
                error!("💡 Check your internet connection and firewall settings");
            }
            
            // Check if it's an authentication issue
            if e.to_string().contains("Unauthenticated") || e.to_string().contains("Invalid") {
                error!("🔐 Authentication Issue Detected");
                error!("💡 Check if your X-Token is valid and not expired");
                error!("💡 Verify your Shyft account status and API limits");
            }
            
            return Err(e);
        }
    };
    
    info!("📋 Building subscribe request...");
    let request = flags.build_subscribe_request();
    info!("   Account filters: {:?}", request.accounts.keys().collect::<Vec<_>>());
    info!("   Commitment level: {:?}", request.commitment);
    if let Some(from_slot) = request.from_slot {
        info!("   Starting from slot: {}", from_slot);
    }
    
    info!("🚀 Starting geyser subscription...");
    geyser_subscribe(client, request, &conn_args.redis_url, flags).await?;
    
    info!("✅ Geyser subscription completed successfully");
    Ok(())
}

async fn geyser_subscribe(
    mut client: GeyserGrpcClient<impl Interceptor>,
    request: SubscribeRequest,
    redis_url: &str,
    flags: &AppArgs,
) -> anyhow::Result<()> {
    let (mut _tx, mut stream) = client.subscribe_with_request(Some(request)).await?;
    info!("stream opened");

    let redis_client = redis::Client::open(redis_url)?;
    let mut redis_conn: MultiplexedConnection = redis_client
        .get_multiplexed_tokio_connection()
        .await?;

    let pong: String = redis::cmd("PING").query_async(&mut redis_conn).await.unwrap_or_else(|_| "PONG".to_string());
    info!("Redis ping: {}", pong);

    // // Test Redis connection with a simple command
    // let pong: String = redis_conn.get("test_connection").await.unwrap_or_else(|_| "connected".to_string());
    // info!("Redis connection test: {}", pong);

    // In-process cache for mint metadata
    let mut mint_cache = MintCache::new(1024);

    // Redis client
    let redis_client = redis::Client::open(redis_url)?;
    let mut redis_conn: MultiplexedConnection = redis_client
        .get_multiplexed_tokio_connection()
        .await?;


    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => match msg.update_oneof {
                Some(UpdateOneof::Account(account)) => {
                    if let Err(e) = handle_account_update(&mut redis_conn, flags, &mut mint_cache, &account ).await {
                        error!("handle_account_update error: {e:?}");
                    }
                }
                Some(UpdateOneof::Ping(_)) => {
                    // no-op; some servers send ping/pong automatically
                }
                _ => {}
            },
            Err(e) => {
                error!("stream error: {e:?}");
                break;
            }
        }
    }

    info!("stream closed");
    Ok(())
}

async fn handle_account_update(
    redis: &mut MultiplexedConnection,
    flags: &AppArgs,
    mint_cache: &mut MintCache,
    upd: &SubscribeUpdateAccount
) -> anyhow::Result<()> {
    let Some(acct) = &upd.account else { return Ok(()); };
    let data: &[u8] = &acct.data;
    if data.len() < 8 { return Ok(()); }

    let disc = &data[..8];

    // Only IDL-parse Whirlpool pools
    if disc == WHIRLPOOL_ACCOUNT_DISCM.as_slice() {
        let whirlpool_acc = WhirlpoolAccount::deserialize(data)
            .map_err(|e| anyhow::anyhow!("WhirlpoolAccount::deserialize: {e}"))?;
        let whirlpool: Whirlpool = whirlpool_acc.0;
    
        let pool_pubkey = bs58::encode(&acct.pubkey).into_string();
        let mint_a = whirlpool.token_mint_a.to_string();
        let mint_b = whirlpool.token_mint_b.to_string();
    
        // Pull decimals/symbols from Redis (w/ in-process LRU)
        let meta_a = mint_cache.get_from_redis(redis, &mint_a).await?;
        let meta_b = mint_cache.get_from_redis(redis, &mint_b).await?;
    
        let pair_symbol = match (&meta_a.symbol, &meta_b.symbol) {
            (Some(sa), Some(sb)) if !sa.is_empty() && !sb.is_empty() => Some(format!("{}/{}", sa, sb)),
            _ => None,
        };
    
        // Write full snapshot including decimals (+ optional pair_symbol)
        cache_whirlpool_snapshot(
            redis,
            &pool_pubkey,
            &whirlpool,
            meta_a.decimals,
            meta_b.decimals,
            pair_symbol.as_deref(),
            upd.slot,
        ).await?;
    
        // Optional verification
        let pool_key = format!("dex:pool:{pool_pubkey}");
        let snap: HashMap<String, String> = redis.hgetall(&pool_key).await?;
        info!("cached {}", pool_key);
        if let (Some(sp), Some(t)) = (snap.get("sqrt_price_x64"), snap.get("tick")) {
            info!("  sqrt_price_x64={} tick={}", sp, t);
        }
        return Ok(());
    }
    

    // Optionally stash TickArrays / Positions
    let now = Utc::now().timestamp();
    if flags.include_tickarrays && disc == TICK_ARRAY_ACCOUNT_DISCM.as_slice() {
        // Try to parse TickArray, but handle deserialization errors gracefully
        match TickArrayAccount::deserialize(data) {
            Ok(tick_array) => {
                let ta = tick_array.0;
                let pool_pubkey = ta.whirlpool.to_string();
                let start_tick = ta.start_tick_index;
                let num_initialized_ticks = ta.ticks.iter().filter(|t| t.initialized).count();
                let sum_gross_liquidity: u128 = ta.ticks.iter().map(|t| t.liquidity_gross).sum();

                let _id: String = redis::cmd("XADD")
                    .arg("dex:whirlpool:tickarray_updates").arg("*")
                    .arg("pool").arg(&pool_pubkey)
                    .arg("array").arg(bs58::encode(&upd.account.as_ref().unwrap().pubkey).into_string())
                    .arg("start").arg(start_tick)
                    .arg("num_init").arg(num_initialized_ticks)
                    .arg("sum_gross").arg(sum_gross_liquidity.to_string())
                    .arg("slot").arg(upd.slot as i64)
                    .arg("ts").arg(now)
                    .query_async(redis).await?;
            }
            Err(e) => {
                // Log the error but still emit basic metadata for depth builder
                warn!("TickArray deserialization failed for {}: {}, emitting basic metadata", bs58::encode(&upd.account.as_ref().unwrap().pubkey).into_string(), e);
                
                // Extract pool ID from account data if possible (first 32 bytes)
                let pool_pubkey = if data.len() >= 32 {
                    bs58::encode(&data[..32]).into_string()
                } else {
                    "unknown".to_string()
                };
                
                let _id: String = redis::cmd("XADD")
                    .arg("dex:whirlpool:tickarray_updates").arg("*")
                    .arg("pool").arg(&pool_pubkey)
                    .arg("array").arg(bs58::encode(&upd.account.as_ref().unwrap().pubkey).into_string())
                    .arg("start").arg(0) // Unknown start tick
                    .arg("num_init").arg(0) // Unknown initialized count
                    .arg("sum_gross").arg("0") // Unknown liquidity
                    .arg("slot").arg(upd.slot as i64)
                    .arg("ts").arg(now)
                    .arg("error").arg("deserialization_failed")
                    .query_async(redis).await?;
            }
        }
        return Ok(());
    }

    if flags.include_positions && disc == POSITION_ACCOUNT_DISCM.as_slice() {
        let _id: String = redis::cmd("XADD")
            .arg("dex:whirlpool:position_updates").arg("*")
            .arg("slot").arg(upd.slot as i64)
            .arg("len").arg(data.len() as i64)
            .arg("ts").arg(now)
            .query_async(redis).await?;
        return Ok(());
    }

    // If --owner-only, you will see many other accounts; ignore by default
    Ok(())
}

/// Writes a single "latest snapshot" for a Whirlpool pool into Redis.
/// Creates the keys if they do not exist (Redis is schemaless).
/// Writes a single \"latest snapshot\" for a Whirlpool pool into Redis.
pub async fn cache_whirlpool_snapshot(
    redis_conn: &mut MultiplexedConnection,
    pool_pubkey: &str,
    whirlpool: &Whirlpool,
    decimals_a: u8,
    decimals_b: u8,
    pair_symbol: Option<&str>,
    slot: u64,
) -> redis::RedisResult<()> {
    let pool_key = format!("dex:pool:{pool_pubkey}");

    let mint_a = whirlpool.token_mint_a.to_string();
    let mint_b = whirlpool.token_mint_b.to_string();
    let fee_bps = whirlpool.fee_rate as i64;
    let sqrt_price_x64 = whirlpool.sqrt_price.to_string();
    let tick = whirlpool.tick_current_index as i64;
    let pair = format!("{mint_a}/{mint_b}");
    let now = Utc::now().timestamp();

    let mut p = pipe();
    p.cmd("HSET").arg(&pool_key)
        .arg("mint_a").arg(&mint_a)
        .arg("mint_b").arg(&mint_b)
        .arg("decimals_a").arg(decimals_a as i64)
        .arg("decimals_b").arg(decimals_b as i64)
        .arg("fee_bps").arg(fee_bps)
        .arg("sqrt_price_x64").arg(sqrt_price_x64)
        .arg("tick").arg(tick)
        .arg("updated_slot").arg(slot as i64)
        .arg("updated_at").arg(now)
        .arg("pair").arg(&pair);

    if let Some(ps) = pair_symbol {
        p.arg("pair_symbol").arg(ps);
        // symbol-based mapping too (if both symbols known)
        p.cmd("SET").arg(format!("dex:pair_symbol:latest_pool:{ps}")).arg(pool_pubkey).ignore();
    }

    // address-based pair mapping
    p.cmd("SET").arg(format!("dex:pair:latest_pool:{pair}")).arg(pool_pubkey).ignore();

    // Use explicit generic for older redis versions; both are fine:
    p.query_async::<()>(&mut *redis_conn).await
}

