//use std::str::FromStr;

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
//use solana_sdk::pubkey::Pubkey;


use {
    backoff::{future::retry, ExponentialBackoff}, clap::Parser as ClapParser, futures::{
        future::TryFutureExt,
        sink::SinkExt,
        stream::StreamExt,
    }, log::{error, info},
        whirlpool_interface::accounts::{WhirlpoolsConfigExtension, WhirlpoolsConfigExtensionAccount,WHIRLPOOLS_CONFIG_EXTENSION_ACCOUNT_DISCM,WhirlpoolsConfig, WhirlpoolsConfigAccount, WHIRLPOOLS_CONFIG_ACCOUNT_DISCM, FeeTier, FeeTierAccount, FEE_TIER_ACCOUNT_DISCM, PositionBundle, PositionBundleAccount,POSITION_BUNDLE_ACCOUNT_DISCM, Position, PositionAccount, POSITION_ACCOUNT_DISCM, TickArray, TickArrayAccount, TICK_ARRAY_ACCOUNT_DISCM, TokenBadge, TokenBadgeAccount, TOKEN_BADGE_ACCOUNT_DISCM, Whirlpool, WhirlpoolAccount, WHIRLPOOL_ACCOUNT_DISCM},
        serde::Serialize, std::{
        collections::{HashMap, HashSet}, env, sync::Arc, time::Duration
    }, tokio::sync::Mutex, tonic::transport::channel::ClientTlsConfig, yellowstone_grpc_client::{GeyserGrpcClient, Interceptor}, yellowstone_grpc_proto::{
        geyser::SubscribeRequestFilterAccounts,
        prelude::{
            subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
        },
    }, solana_program::pubkey::Pubkey
};
use solana_client::rpc_client::RpcClient;
use axum::{routing::get, Router, http::StatusCode, response::Json};
use tokio::net::TcpListener;
use redis::{aio::MultiplexedConnection, AsyncCommands};
mod helpers;
use helpers::{
    derive_tick_array_address,
    marginal_price_b_per_a,
    q64_to_f64,
    size_to_next_tick_in_estimate_ab,
    size_to_next_tick_in_estimate_ba,
    three_tick_arrays_around,
    tick_to_sqrt_price_x64,
};
use crate::helpers::seed_tick_arrays_via_rpc;
use std::str::FromStr;

type AccountFilterMap = HashMap<String, SubscribeRequestFilterAccounts>;

const WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

#[derive(Debug, Clone, ClapParser)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, env = "ENDPOINT", help = "gRPC endpoint (or ENDPOINT env)")]
    endpoint: String,

    #[clap(long, env = "X_TOKEN", help = "X-Token (or X_TOKEN env)")]
    x_token: String,
}



impl Args {
    async fn connect(&self) -> anyhow::Result<GeyserGrpcClient<impl Interceptor>> {
        GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .x_token(Some(self.x_token.clone()))?
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(20))
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .max_decoding_message_size(1024 * 1024 * 1024)
            .connect()
            .await
            .map_err(Into::into)
    }

    pub fn get_txn_updates(&self) -> anyhow::Result<SubscribeRequest> {
        

        let mut accounts: AccountFilterMap = HashMap::new();

        accounts.insert(
            "accountData".to_owned(),
            SubscribeRequestFilterAccounts {
                account: vec![],
                owner: vec![WHIRLPOOL_PROGRAM_ID.to_string()],
                nonempty_txn_signature: None,
                filters: vec![]
            },
        );

        Ok(SubscribeRequest {
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
            from_slot: None,
        })
    }
}


pub trait AccountData: std::fmt::Debug {}

#[derive(Debug,Default)]
pub struct EmptyAccount;

impl AccountData for EmptyAccount {}

#[derive(Debug, Serialize)]
pub enum DecodedAccount {
    WhirlpoolsConfig(WhirlpoolsConfig),
    WhirlpoolsConfigExtension(WhirlpoolsConfigExtension),
    FeeTier(FeeTier),
    PositionBundle(PositionBundle),
    Position(Position),
    TickArray(TickArray),
    TokenBadge(TokenBadge),
    Whirlpool(Whirlpool),
}

#[derive(Debug)]
pub struct AccountDecodeError {
    pub message: String,
}



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env::set_var(
        env_logger::DEFAULT_FILTER_ENV,
        env::var_os(env_logger::DEFAULT_FILTER_ENV).unwrap_or_else(|| "info".into()),
    );
    env_logger::init();

    let args = Args::parse();

    // Health/metrics server
    let bind = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("HEALTH_PORT").unwrap_or_else(|_| "8093".to_string()).parse().unwrap_or(8093);
    tokio::spawn(async move {
        let app = Router::new()
            .route("/health", get(|| async { (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))) }))
            .route("/metrics", get(|| async { (StatusCode::OK, Json(serde_json::json!({"status":"ready"}))) }));
        let addr = format!("{}:{}", bind, port);
        if let Ok(listener) = TcpListener::bind(&addr).await {
            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("health server error: {}", e);
            }
        } else {
            eprintln!("failed to bind health server on {}", addr);
        }
    });
    let zero_attempts = Arc::new(Mutex::new(true));

    // The default exponential backoff strategy intervals:
    // [500ms, 750ms, 1.125s, 1.6875s, 2.53125s, 3.796875s, 5.6953125s,
    // 8.5s, 12.8s, 19.2s, 28.8s, 43.2s, 64.8s, 97s, ... ]
    retry(ExponentialBackoff::default(), move || {
        let args = args.clone();
        let zero_attempts = Arc::clone(&zero_attempts);

        async move {
            let mut zero_attempts = zero_attempts.lock().await;
            if *zero_attempts {
                *zero_attempts = false;
            } else {
                info!("Retry to connect to the server");
            }
            drop(zero_attempts);

            let client = args.connect().await.map_err(backoff::Error::transient)?;
            info!("Connected");

            let request = args.get_txn_updates().map_err(backoff::Error::Permanent)?;

            geyser_subscribe(client, request)
                .await
                .map_err(backoff::Error::transient)?;

            Ok::<(), backoff::Error<anyhow::Error>>(())
        }
        .inspect_err(|error| error!("failed to connect: {error}"))
    })
    .await
    .map_err(Into::into)
}



async fn geyser_subscribe(
    mut client: GeyserGrpcClient<impl Interceptor>,
    request: SubscribeRequest,
) -> anyhow::Result<()> {
    let (mut subscribe_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;

    info!("stream opened");

    // Optional Redis connection for outputs
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379/0".to_string());
    let redis_client = redis::Client::open(redis_url).ok();
    let mut redis_conn: Option<MultiplexedConnection> = if let Some(c) = &redis_client { c.get_multiplexed_async_connection().await.ok() } else { None };

    // Track pools we've subscribed tick arrays for, to avoid duplicate requests
    let mut subscribed_pools: HashSet<String> = HashSet::new();
    // Map pool_pubkey -> tick_spacing
    let mut pool_tick_spacing: HashMap<String, i32> = HashMap::new();
    // Map tick_array_address -> (pool_pubkey, expected_start)
    let mut ta_expect: HashMap<String, (String, i32)> = HashMap::new();
    // Pools we have seeded via RPC (tick arrays fetched once)
    let mut seeded_pools: HashSet<String> = HashSet::new();
    // Optional RPC client for seeding (non-hot-path)
    let seeding_enabled = std::env::var("SEED_TICK_ARRAYS").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
    let rpc_client: Option<RpcClient> = if seeding_enabled {
        if let Ok(url) = std::env::var("RPC_URL") {
            Some(RpcClient::new_with_timeout(url, Duration::from_secs(10)))
        } else {
            None
        }
    } else {
        None
    };

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => match msg.update_oneof {
                
                Some(UpdateOneof::Account(account)) => {
                    
                    let slot = account.slot;
                    //let account_update = account.account;
                    if let Some(account_data) = account.account {
                        let pubkey_str = bs58::encode(&account_data.pubkey).into_string();
                        let owner = bs58::encode(&account_data.owner).into_string();
                        let lamports = account_data.lamports;
                        let executable = account_data.executable;
                
                        let decoded_account = match decode_account_data(&account_data.data) {
                            Ok(data) => data,
                            Err(e) => {
                                eprintln!("Failed 2to decode account data: {}", e.message);
                                continue;// Handle the error as needed
                            }
                        };
                
                        info!("account slot={} pubkey={} owner={} exec={} len={}", slot, pubkey_str, owner, executable, account_data.data.len());

                        // If this account is one of the expected TickArrays, run validation/logging
                        // Decode and publish TickArray snapshot to Redis regardless of expected map
                        if account_data.data.len() >= 8 && &account_data.data[..8] == &TICK_ARRAY_ACCOUNT_DISCM {
                            if let Ok(ta) = TickArrayAccount::deserialize(&account_data.data) {
                                let pool_pk = ta.0.whirlpool;
                                let expected_start = ta.0.start_tick_index;
                                let pool_b58 = bs58::encode(pool_pk).into_string();
                                let spacing = pool_tick_spacing.get(&pool_b58).cloned().unwrap_or(0);
                                info!("tick_array addr={} len={} start={} pool={} spacing={}", pubkey_str, account_data.data.len(), expected_start, pool_b58, spacing);
                                if let Some(conn) = redis_conn.as_mut() {
                                    let mut idxs = Vec::new();
                                    for (i, t) in ta.0.ticks.iter().enumerate() {
                                        if t.liquidity_gross > 0 {
                                            let idx_tick = expected_start + (i as i32) * spacing.max(1);
                                            idxs.push(idx_tick);
                                        }
                                    }
                                    let key = format!("whirlpool:tickarray:{}:{}", expected_start, pool_b58);
                                    let payload = serde_json::json!({
                                        "pool": pool_b58,
                                        "tick_array": pubkey_str,
                                        "expected_start": expected_start,
                                        "tick_spacing": spacing,
                                        "slot": slot,
                                        "initialized_count": idxs.len(),
                                        "initialized_indices": idxs,
                                    });
                                    let _ : Result<(), _> = conn.set_ex(key, payload.to_string(), 180).await;
                                }
                            }
                        }

                        // If this is a Whirlpool account, derive and subscribe to its three TickArrays
                        if let DecodedAccount::Whirlpool(ref pool) = decoded_account {
                            let pool_pubkey = match account_data.pubkey.as_slice().try_into() as Result<[u8;32], _> {
                                Ok(arr) => Pubkey::new_from_array(arr),
                                Err(_) => Pubkey::default(),
                            };
                            let tick_spacing = pool.tick_spacing as i32;
                            pool_tick_spacing.insert(bs58::encode(pool_pubkey).into_string(), tick_spacing);
                            let total_fee_bps = pool.fee_rate as u32 + pool.protocol_fee_rate as u32;
                            let price_b_per_a = marginal_price_b_per_a(pool.sqrt_price, 0, 0);
                            info!(
                                "whirlpool slot={} pool={} tick={} spacing={} fee_lp_bps={} protocol_fee_bps={} total_fee_bps={} price_b_per_a={}",
                                slot,
                                bs58::encode(pool_pubkey).into_string(),
                                pool.tick_current_index,
                                tick_spacing,
                                pool.fee_rate,
                                pool.protocol_fee_rate,
                                total_fee_bps,
                                price_b_per_a
                            );
                            let ticks_per_array: i32 = 88;
                            let span = tick_spacing * ticks_per_array;
                            let curr_tick = pool.tick_current_index;
                            // floor division for negatives handled via div_euclid
                            let current_start = (curr_tick).div_euclid(span) * span;
                            let lower_start = current_start - span;
                            let upper_start = current_start + span;

                            let program_id = Pubkey::from_str(WHIRLPOOL_PROGRAM_ID).unwrap_or_else(|_| Pubkey::default());

                            let derive_ta = |start: i32| {
                                let le = start.to_le_bytes();
                                Pubkey::find_program_address(&[b"tick_array", pool_pubkey.as_ref(), &le], &program_id).0
                            };

                            let ta_lower = derive_ta(lower_start);
                            let ta_current = derive_ta(current_start);
                            let ta_upper = derive_ta(upper_start);

                            let pool_key_str = bs58::encode(pool_pubkey).into_string();
                            // Seed the three TickArrays once via RPC (if enabled and not yet seeded), and publish snapshots
                            if seeding_enabled && !seeded_pools.contains(&pool_key_str) {
                                if let Some(ref rpc) = rpc_client {
                                    let addrs = vec![ta_lower, ta_current, ta_upper];
                                    if let Ok(accounts) = rpc.get_multiple_accounts(&addrs) {
                                        for (idx, acc_opt) in accounts.into_iter().enumerate() {
                                            if let Some(acc) = acc_opt {
                                                let data = acc.data.as_slice();
                                                if data.len() >= 8 && &data[..8] == &TICK_ARRAY_ACCOUNT_DISCM {
                                                    if let Ok(ta) = TickArrayAccount::deserialize(data) {
                                                        let start = ta.0.start_tick_index;
                                                        let mut idxs = Vec::new();
                                                        for (i, t) in ta.0.ticks.iter().enumerate() {
                                                            if t.liquidity_gross > 0 {
                                                                let idx_tick = start + (i as i32) * tick_spacing;
                                                                idxs.push(idx_tick);
                                                            }
                                                        }
                                                        if let Some(conn) = redis_conn.as_mut() {
                                                            let key = format!("whirlpool:tickarray:{}:{}", start, pool_key_str);
                                                            let payload = serde_json::json!({
                                                                "pool": pool_key_str,
                                                                "tick_array": bs58::encode(addrs[idx]).into_string(),
                                                                "expected_start": start,
                                                                "tick_spacing": tick_spacing,
                                                                "slot": slot,
                                                                "initialized_count": idxs.len(),
                                                                "initialized_indices": idxs,
                                                            });
                                                            let _ : Result<(), _> = conn.set_ex(key, payload.to_string(), 180).await;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    seeded_pools.insert(pool_key_str.clone());
                                }
                            }
                            info!(
                                "tick_pdas span={} spacing={} starts=({}, {}, {}) lower={} current={} upper={}",
                                span,
                                tick_spacing,
                                lower_start,
                                current_start,
                                upper_start,
                                bs58::encode(ta_lower).into_string(),
                                bs58::encode(ta_current).into_string(),
                                bs58::encode(ta_upper).into_string(),
                            );
                            // Publish pool snapshot to Redis (TTL 180s). Use strings for 128-bit ints.
                            if let Some(conn) = redis_conn.as_mut() {
                                let key = format!("whirlpool:pool:{}", bs58::encode(&pool_pubkey).into_string());
                                let payload = serde_json::json!({
                                    "pool": bs58::encode(&pool_pubkey).into_string(),
                                    "program": WHIRLPOOL_PROGRAM_ID,
                                    "token_a": bs58::encode(pool.token_mint_a).into_string(),
                                    "token_b": bs58::encode(pool.token_mint_b).into_string(),
                                    "sqrt_price_x64": pool.sqrt_price.to_string(),
                                    "liquidity": pool.liquidity.to_string(),
                                    "tick_current": pool.tick_current_index,
                                    "tick_spacing": tick_spacing,
                                    "fee_bps": pool.fee_rate,
                                    "protocol_fee_bps": pool.protocol_fee_rate,
                                    "slot": slot,
                                    "tick_array_starts": [lower_start, current_start, upper_start],
                                });
                                let _ : Result<(), _> = conn.set_ex(key, payload.to_string(), 180).await;
                            }
                            if !subscribed_pools.contains(&pool_key_str) {
                                let mut accs = HashMap::new();
                                accs.insert(
                                    format!("tick_arrays_{}", &pool_key_str),
                                    SubscribeRequestFilterAccounts {
                                        account: vec![
                                            bs58::encode(ta_lower).into_string(),
                                            bs58::encode(ta_current).into_string(),
                                            bs58::encode(ta_upper).into_string(),
                                        ],
                                        owner: vec![],
                                        nonempty_txn_signature: None,
                                        filters: vec![],
                                    },
                                );

                                // Record expected starts for later validation when tick arrays arrive
                                ta_expect.insert(bs58::encode(ta_lower).into_string(), (pool_key_str.clone(), lower_start));
                                ta_expect.insert(bs58::encode(ta_current).into_string(), (pool_key_str.clone(), current_start));
                                ta_expect.insert(bs58::encode(ta_upper).into_string(), (pool_key_str.clone(), upper_start));

                                let req = SubscribeRequest {
                                    accounts: accs,
                                    slots: HashMap::default(),
                                    transactions: HashMap::default(),
                                    transactions_status: HashMap::default(),
                                    blocks: HashMap::default(),
                                    blocks_meta: HashMap::default(),
                                    entry: HashMap::default(),
                                    commitment: Some(CommitmentLevel::Processed as i32),
                                    accounts_data_slice: Vec::default(),
                                    ping: None,
                                    from_slot: None,
                                };
                                if let Err(e) = subscribe_tx.send(req).await {
                                    error!("failed to send tick array subscribe request: {e}");
                                } else {
                                    info!(
                                        "subscribed tick arrays for pool {} (starts: {}, {}, {})",
                                        pool_key_str, lower_start, current_start, upper_start
                                    );
                                    subscribed_pools.insert(pool_key_str);
                                }
                            }
                        }
                    } else {
                        info!("Account data is None for slot: {}", slot);
                    }

                    

                }
                Some(UpdateOneof::Ping(_)) => {
                    subscribe_tx
                        .send(SubscribeRequest {
                            ping: Some(SubscribeRequestPing { id: 1 }),
                            ..Default::default()
                        })
                        .await?;
                }
                Some(UpdateOneof::Pong(_)) => {}
                None => {
                    error!("update not found in the message");
                    break;
                }
                _ => {}
            },
            Err(error) => {
                error!("error: {error:?}");
                break;
            }
        }
    }

    info!("stream closed");
    Ok(())
}

pub fn decode_account_data(buf: &[u8]) -> Result<DecodedAccount, AccountDecodeError> {
    if buf.len() < 8 {
        return Err(AccountDecodeError {
            message: "Buffer too short to contain a valid discriminator.".to_string(),
        });
    }

    let discriminator: [u8; 8] = buf[..8].try_into().expect("Failed to extract first 8 bytes");

    match discriminator {
        WHIRLPOOLS_CONFIG_ACCOUNT_DISCM => {
            //println!("Whirlpool Config Account detected. Proceeding with deserialization...");
            let data = WhirlpoolsConfigAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize WhirlpoolsConfigAccount: {}", e),
                })?;
            info!("decoded whirlpool_config");
            Ok(DecodedAccount::WhirlpoolsConfig(data.0)) // Unwrapping the inner BondingCurve struct
        }
        WHIRLPOOLS_CONFIG_EXTENSION_ACCOUNT_DISCM => {
            let data = WhirlpoolsConfigExtensionAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize WhirlpoolsConfigExtAccount: {}", e),
                })?;

            info!("decoded whirlpool_config_extension");
            Ok(DecodedAccount::WhirlpoolsConfigExtension(data.0)) // Unwrapping the inner BondingCurve struct
        }
        FEE_TIER_ACCOUNT_DISCM => {
            let data = FeeTierAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize FreeTier Account: {}", e),
                })?;

            info!("decoded fee_tier");
            Ok(DecodedAccount::FeeTier(data.0)) // Unwrapping the inner BondingCurve struct
        }
        POSITION_BUNDLE_ACCOUNT_DISCM => {
            let data = PositionBundleAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize PositionBundleAccount: {}", e),
                })?;
            info!("decoded position_bundle");
            Ok(DecodedAccount::PositionBundle(data.0)) // Unwrapping the inner BondingCurve struct
        }
        POSITION_ACCOUNT_DISCM => {
            let data = PositionAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize PositionAccount: {}", e),
                })?;
            info!("decoded position");
            Ok(DecodedAccount::Position(data.0)) // Unwrapping the inner BondingCurve struct
        }
        TICK_ARRAY_ACCOUNT_DISCM => {
            let data = TickArrayAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize TickArrayAccount: {}", e),
                })?;
            info!("decoded tick_array");
            Ok(DecodedAccount::TickArray(data.0)) // Unwrapping the inner BondingCurve struct
        }
        TOKEN_BADGE_ACCOUNT_DISCM => {
            let data = TokenBadgeAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize TokenBadgeAccount: {}", e),
                })?;
            info!("decoded token_badge");
            Ok(DecodedAccount::TokenBadge(data.0)) // Unwrapping the inner BondingCurve struct
        }
        WHIRLPOOL_ACCOUNT_DISCM => {
            let data = WhirlpoolAccount::deserialize(buf)
                .map_err(|e| AccountDecodeError {
                    message: format!("Failed to deserialize WhirlpoolAccount: {}", e),
                })?;
            info!("decoded whirlpool");
            Ok(DecodedAccount::Whirlpool(data.0)) // Unwrapping the inner BondingCurve struct
        }
        _ => Err(AccountDecodeError {
            message: "Account discriminator not found.".to_string(),
        }),
    }
}


// -------------------- Anchor discriminator helpers --------------------

fn anchor_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

// Orca Whirlpool account discriminators (compute at runtime to avoid mismatches)
fn whirlpool_disc() -> [u8; 8] {
    anchor_discriminator("account:Whirlpool")
}
fn tick_array_disc() -> [u8; 8] {
    anchor_discriminator("account:TickArray")
}


// -------------------- On-chain layouts (use types from whirlpool_interface) --------------------

// -------------------- Local experimental on-chain layouts (kept, renamed to avoid conflicts) --------------------
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct TickLocal {
    pub initialized: bool,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_a: u128,
    pub fee_growth_outside_b: u128,
    pub reward_growths_outside: [u128; 3],
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct TickArrayLocal {
    pub start_tick_index: i32,
    pub whirlpool: Pubkey,
    pub ticks: [TickLocal; 88],
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct RewardInfoLocal {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub emissions_per_second_x64: u128,
    pub growth_global_x64: u128,
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct WhirlpoolMinimalLocal {
    pub whirlpools_config: Pubkey,
    pub whirlpool_bump: [u8; 1],
    pub tick_spacing: u16,
    pub tick_spacing_seed: [u8; 2],
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub fee_growth_global_a: u128,
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub fee_growth_global_b: u128,
    pub reward_last_updated_timestamp: u64,
    pub reward_infos: [RewardInfoLocal; 3],
}

// Local decode helpers (do not affect main decode path)
pub fn decode_whirlpool_local(data: &[u8]) -> Result<WhirlpoolMinimalLocal> {
    let disc = whirlpool_disc();
    if data.len() < 8 || data[..8] != disc {
        return Err(anyhow!("wrong discriminator for Whirlpool"));
    }
    let mut cursor = &data[8..];
    Ok(WhirlpoolMinimalLocal::deserialize(&mut cursor)?)
}

pub fn decode_tick_array_local(data: &[u8]) -> Result<TickArrayLocal> {
    let disc = tick_array_disc();
    if data.len() < 8 || data[..8] != disc {
        return Err(anyhow!("wrong discriminator for TickArray"));
    }
    let mut cursor = &data[8..];
    Ok(TickArrayLocal::deserialize(&mut cursor)?)
}

// -------------------- PDA derivation --------------------

// Backwards-compatible thin wrapper to keep existing call sites working
pub fn decode_tick_array(data: &[u8]) -> anyhow::Result<TickArray> {
    let disc = TICK_ARRAY_ACCOUNT_DISCM;
    if data.len() < 8 || &data[..8] != &disc {
        return Err(anyhow!("wrong discriminator for TickArray"));
    }
    let parsed = TickArrayAccount::deserialize(data)
        .map_err(|e| anyhow!("deserialize TickArrayAccount failed: {e}"))?;
    Ok(parsed.0)
}

// -------------------- Math helpers (logging / first estimates) --------------------

// -------------------- Example glue (use with your stream) --------------------

#[derive(Debug)]
pub struct PoolSnapshot {
    pub program_id: Pubkey,
    pub pool: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub tick_current: i32,
    pub tick_spacing: i32,
    pub fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub slot: u64,
}





pub fn on_whirlpool_update(pool: &PoolSnapshot) {
    let total_fee_bps = pool.fee_bps as u32 + pool.protocol_fee_bps as u32;

// 1) Log marginal price (for sanity)
let p_b_per_a = marginal_price_b_per_a(
    pool.sqrt_price_x64,
    pool.decimals_a as i32,
    pool.decimals_b as i32,
);
println!(
    "slot={} pool={} tick={} spacing={} fee_total_bps={} price(B/A)={}",
    pool.slot,
    pool.pool,
    pool.tick_current,
    pool.tick_spacing,
    total_fee_bps,
    p_b_per_a
);

// 2) Derive the three tick-array PDAs you should subscribe to
let (lower_start, current_start, upper_start) =
    three_tick_arrays_around(pool.tick_current, pool.tick_spacing);
let (ta_lower, _) = derive_tick_array_address(&pool.program_id, &pool.pool, lower_start);
let (ta_current, _) = derive_tick_array_address(&pool.program_id, &pool.pool, current_start);
let (ta_upper, _) = derive_tick_array_address(&pool.program_id, &pool.pool, upper_start);

println!(
    "tick_arrays: lower({}) current({}) upper({}) starts=({}, {}, {})",
    ta_lower, ta_current, ta_upper, lower_start, current_start, upper_start
);

// You should now ensure your stream/RPC is fetching these 3 accounts
// and passing their data into on_tick_array_update below.
}


// Call this when you receive a TickArray account update (data is the raw account.data)
pub fn on_tick_array_update(
    pool_pubkey: &Pubkey,
    expected_start: i32,
    tick_spacing: i32,
    data: &[u8],
) -> Result<()> {
    let ta = decode_tick_array(data)?;
    // Sanity checks
    if ta.whirlpool != *pool_pubkey {
        return Err(anyhow!("tick_array whirlpool mismatch"));
    }
    if ta.start_tick_index != expected_start {
        return Err(anyhow!(
            "tick_array start mismatch: got {}, expected {}",
            ta.start_tick_index,
            expected_start
        ));
    }
    let init_count = ta.ticks.iter().filter(|t| t.liquidity_gross > 0).count();
    println!(
        "[TICK_ARRAY] start_tick_index={} whirlpool_ok={} initialized_ticks={}",
        ta.start_tick_index, ta.whirlpool == *pool_pubkey, init_count
    );
    // Print first few initialized tick indices
    let mut printed = 0;
    for (i, t) in ta.ticks.iter().enumerate() {
        if t.liquidity_gross > 0 {
            let tick_index = ta.start_tick_index + (i as i32) * tick_spacing;
            println!("  initialized at offset {} (raw index {})", i, tick_index);
            printed += 1;
            if printed >= 5 {
                break;
            }
        }
    }
    Ok(())
}


// Example: compute rough size_to_next_tick for both directions given current tick-array window
pub fn estimate_sizes_to_next_tick(
    pool: &PoolSnapshot,
    // supply the nearest initialized tick indices on each side (you can scan ta_current and neighbors)
    next_lower_initialized_tick: i32,
    next_upper_initialized_tick: i32,
) {
    let total_fee_bps = pool.fee_bps as u32 + pool.protocol_fee_bps as u32;

let size_ab = size_to_next_tick_in_estimate_ab(
    pool.sqrt_price_x64,
    pool.liquidity,
    pool.tick_current,
    next_lower_initialized_tick,
    total_fee_bps,
);
let size_ba = size_to_next_tick_in_estimate_ba(
    pool.sqrt_price_x64,
    pool.liquidity,
    pool.tick_current,
    next_upper_initialized_tick,
    total_fee_bps,
);

println!(
    "size_to_next_tick_in: A->B ~{} (raw token A units), B->A ~{} (raw token B units)",
    size_ab, size_ba
);
}



// -------------------- Main --------------------

fn _main() {
    // This file is intended as a library of helpers.
    // Wire on_whirlpool_update and on_tick_array_update into your Geyser/Kafka consumers.
    println!("Whirlpool tick-array decode helpers loaded.");
}



/*


How to use this right now

In your Geyser/Kafka consumer:

When you get a Whirlpool pool account update you already decode, build a PoolSnapshot and call on_whirlpool_update(&snapshot). That prints marginal price and the three TickArray PDAs (lower/current/upper) you should subscribe to.

Ensure your stream fetches those three tick-array addresses. When you receive an update for any of them, call on_tick_array_update(&pool_pubkey, expected_start, account.data). expected_start is one of lower_start/current_start/upper_start you computed.

After you can decode tick arrays and count initialized ticks, scan for the nearest initialized tick below and above tick_current_index and call estimate_sizes_to_next_tick to get rough capacities to the next boundary.


Notes

The TickArray layout must be [Tick; 88] and whirlpool before ticks. Do not use Vec.

The discriminator check prevents “Account discriminator not found” issues when you accidentally decode the wrong account type.

The tick index printed in on_tick_array_update uses a raw offset; you need to multiply by tick_spacing when computing actual tick indices:

actual_tick_index = start_tick_index + i * tick_spacing


The math helpers here use f64 for simplicity to get you unstuck. We’ll replace them with integer Q64.64 math in the next step once you confirm decoding is correct and you can see initialized ticks.


*/