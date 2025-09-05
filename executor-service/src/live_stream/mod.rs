//! Live edge stream integration (Whirlpool, Raydium CLMM/CPMM, Meteora DLMM, Phoenix)
//! Additive, opt-in via ENABLE_LIVE_EDGE_STREAM=1

use anyhow::Result;
use futures_util::StreamExt; // for .next() on subscription stream
use log::{error, info, warn};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::SubscribeRequestFilterAccounts;
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
};

// Whirlpool decode
use whirlpool_core::decode::decode_whirlpool;
// (Removed unused WhirlpoolMinimal import)

// Minimal Raydium CLMM PoolState subset (Borsh) — fields we need.
// Discriminator constant reused from archived code.
const RAYDIUM_POOL_STATE_ACCOUNT_DISCM: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];
use borsh::BorshDeserialize;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, BorshDeserialize, Clone)]
struct RaydiumClmmPoolStateMinimal {
    // Keep ordering identical to on-chain struct; include only required prefix fields until those we read.
    pub amm_config: [u8; 32],      // Pubkey
    pub owner: [u8; 32],           // Pubkey
    pub token_mint0: [u8; 32],     // Pubkey
    pub token_mint1: [u8; 32],     // Pubkey
    pub token_vault0: [u8; 32],    // Pubkey
    pub token_vault1: [u8; 32],    // Pubkey
    pub observation_key: [u8; 32], // Pubkey
    pub mint_decimals_0: u8,
    pub mint_decimals_1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub protocol_fees_token0: u64,
    pub protocol_fees_token1: u64,
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
    // Skip rest (borsh still needs layout; to avoid mismatch we must include intervening fields) — risk: layout drift.
}

fn decode_raydium_clmm(data: &[u8]) -> Option<RaydiumClmmPoolStateMinimal> {
    if data.len() < 8 || data[..8] != RAYDIUM_POOL_STATE_ACCOUNT_DISCM {
        return None;
    }
    let mut cursor = &data[8..];
    RaydiumClmmPoolStateMinimal::deserialize(&mut cursor).ok()
}

// -------- Raydium AmmConfig decoding (two observed layouts) --------
const RAYDIUM_AMM_CONFIG_ACCOUNT_DISCM: [u8; 8] = [218, 244, 33, 104, 203, 203, 43, 111];
#[allow(dead_code)]
#[derive(Debug, BorshDeserialize, Clone)]
struct RaydiumAmmConfigV1 {
    pub bump: u8,
    pub index: u16,
    pub owner: [u8; 32],
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub tick_spacing: u16,
    pub fund_fee_rate: u32,
    pub padding_u32: u32,
    pub fund_owner: [u8; 32],
    pub padding: [u64; 3],
}
#[allow(dead_code)]
#[derive(Debug, BorshDeserialize, Clone)]
struct RaydiumAmmConfigLegacy {
    pub bump: u8,
    pub disable_create_pool: bool,
    pub index: u16,
    pub trade_fee_rate: u64,
    pub protocol_fee_rate: u64,
    pub fund_fee_rate: u64,
    pub create_pool_fee: u64,
    pub protocol_owner: [u8; 32],
    pub fund_owner: [u8; 32],
    pub padding: [u64; 16],
}
fn decode_raydium_amm_config(data: &[u8]) -> Option<(u32, u32, u32)> {
    if data.len() < 8 || data[..8] != RAYDIUM_AMM_CONFIG_ACCOUNT_DISCM {
        return None;
    }
    // Attempt V1 (u32 layout)
    let mut c1 = &data[8..];
    if let Ok(v1) = RaydiumAmmConfigV1::deserialize(&mut c1) {
        return Some((v1.trade_fee_rate, v1.protocol_fee_rate, v1.fund_fee_rate));
    }
    // Fallback to legacy (u64) layout
    let mut c2 = &data[8..];
    if let Ok(l) = RaydiumAmmConfigLegacy::deserialize(&mut c2) {
        return Some((
            l.trade_fee_rate as u32,
            l.protocol_fee_rate as u32,
            l.fund_fee_rate as u32,
        ));
    }
    None
}

// Program IDs (some duplicated in other crates; repeated here locally for now).
// TODO: unify via shared-core constants (some mismatch variants present there) once stabilized.
const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"; // without trailing variant chars
const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Unified minimal snapshot for edge derivation
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum LivePoolSnapshot {
    RaydiumClmm {
        pool: String,
        token_a: String,
        token_b: String,
        fee_bps: u32,
        sqrt_price_x64: u128,
        liquidity: u128,
        tick_current: i32,
        slot: u64,
        ts_ns: u64,
        decimals_a: u8,
        decimals_b: u8,
    },
    OrcaWhirlpool {
        pool: String,
        token_a: String,
        token_b: String,
        fee_bps: u32,
        sqrt_price_x64: u128,
        liquidity: u128,
        tick_current: i32,
        tick_spacing: u16,
        slot: u64,
        ts_ns: u64,
        decimals_a: u8,
        decimals_b: u8,
    },
    RaydiumCpmm {
        pool: String,
        token_a: String,
        token_b: String,
        fee_bps: u32,
        reserve_a: u128,
        reserve_b: u128,
        slot: u64,
        ts_ns: u64,
        decimals_a: u8,
        decimals_b: u8,
    },
    MeteoraDlmm {
        pool: String,
        token_a: String,
        token_b: String,
        fee_bps: u32,
        bin_price_q64: u128,
        active_bin: i32,
        depth_hint_in: u128,
        slot: u64,
        ts_ns: u64,
        decimals_a: u8,
        decimals_b: u8,
    },
    PhoenixL2 {
        market: String,
        base: String,
        quote: String,
        best_bid_px_q64: u128,
        best_ask_px_q64: u128,
        slot: u64,
        ts_ns: u64,
        base_decimals: u8,
        quote_decimals: u8,
    },
}

use edge_assembler::edge::{edge_delta, ClmmState, EdgeDelta};

fn hash_edge_id(venue_id: u32, pool: &str, direction: u32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    venue_id.hash(&mut h);
    pool.hash(&mut h);
    direction.hash(&mut h);
    h.finish()
}

#[inline]
fn directional_prices_q64(sqrt_price_x64: u128) -> (u128, u128) {
    if let Some(prod) = sqrt_price_x64.checked_mul(sqrt_price_x64) {
        let price_ab = prod >> 64; // Q64 price of token_b per token_a (assuming sqrt is price_ab^0.5 * 2^64)
        if price_ab > 0 {
            let inv = (u128::MAX / price_ab); // coarse inverse approximation
            (price_ab, inv)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    }
}

/// Convert a snapshot into zero, one, or two EdgeDeltas (both swap directions for CLMM pools)
pub fn snapshot_to_edges(s: &LivePoolSnapshot) -> Vec<EdgeDelta> {
    match s {
        LivePoolSnapshot::OrcaWhirlpool {
            pool,
            token_a,
            token_b,
            fee_bps,
            sqrt_price_x64,
            liquidity,
            tick_current,
            tick_spacing,
            slot,
            ts_ns,
            decimals_a,
            decimals_b,
        } => {
            let venue_id = 1u32; // OrcaWhirlpool
            let pool_bytes = bs58::decode(pool).into_vec().unwrap_or_default();
            let token_a_bytes = bs58::decode(token_a).into_vec().unwrap_or_default();
            let token_b_bytes = bs58::decode(token_b).into_vec().unwrap_or_default();
            let (price_ab, price_ba) = directional_prices_q64(*sqrt_price_x64);
            let state = |dir: u32| EdgeDelta {
                edge_id: hash_edge_id(venue_id, pool, dir),
                venue_id,
                pool_pubkey: pool_bytes.clone(),
                token_in: if dir == 0 {
                    token_a_bytes.clone()
                } else {
                    token_b_bytes.clone()
                },
                token_out: if dir == 0 {
                    token_b_bytes.clone()
                } else {
                    token_a_bytes.clone()
                },
                direction: dir,
                decimals_in: if dir == 0 {
                    *decimals_a as u32
                } else {
                    *decimals_b as u32
                },
                decimals_out: if dir == 0 {
                    *decimals_b as u32
                } else {
                    *decimals_a as u32
                },
                fee_bps_total: *fee_bps,
                slot: *slot,
                write_version: 0,
                updated_at_ns: *ts_ns,
                marginal_price_q64: if dir == 0 {
                    price_ab.to_le_bytes().to_vec()
                } else {
                    price_ba.to_le_bytes().to_vec()
                },
                size_to_next_boundary_in: 0u128.to_le_bytes().to_vec(), // placeholder
                cu_estimate: 50_000,
                state: Some(edge_delta::State::Clmm(ClmmState {
                    sqrt_price_x64: sqrt_price_x64.to_le_bytes().to_vec(),
                    liquidity: liquidity.to_le_bytes().to_vec(),
                    tick_current: *tick_current,
                    tick_spacing: *tick_spacing as i32,
                    size_to_next_in: 0u128.to_le_bytes().to_vec(),
                })),
            };
            vec![state(0), state(1)]
        }
        LivePoolSnapshot::RaydiumClmm {
            pool,
            token_a,
            token_b,
            fee_bps,
            sqrt_price_x64,
            liquidity,
            tick_current,
            slot,
            ts_ns,
            decimals_a,
            decimals_b,
        } => {
            let venue_id = 5u32; // RaydiumClmm
            let pool_bytes = bs58::decode(pool).into_vec().unwrap_or_default();
            let token_a_bytes = bs58::decode(token_a).into_vec().unwrap_or_default();
            let token_b_bytes = bs58::decode(token_b).into_vec().unwrap_or_default();
            let (price_ab, price_ba) = directional_prices_q64(*sqrt_price_x64);
            let state = |dir: u32| EdgeDelta {
                edge_id: hash_edge_id(venue_id, pool, dir),
                venue_id,
                pool_pubkey: pool_bytes.clone(),
                token_in: if dir == 0 {
                    token_a_bytes.clone()
                } else {
                    token_b_bytes.clone()
                },
                token_out: if dir == 0 {
                    token_b_bytes.clone()
                } else {
                    token_a_bytes.clone()
                },
                direction: dir,
                decimals_in: if dir == 0 {
                    *decimals_a as u32
                } else {
                    *decimals_b as u32
                },
                decimals_out: if dir == 0 {
                    *decimals_b as u32
                } else {
                    *decimals_a as u32
                },
                fee_bps_total: *fee_bps,
                slot: *slot,
                write_version: 0,
                updated_at_ns: *ts_ns,
                marginal_price_q64: if dir == 0 {
                    price_ab.to_le_bytes().to_vec()
                } else {
                    price_ba.to_le_bytes().to_vec()
                },
                size_to_next_boundary_in: 0u128.to_le_bytes().to_vec(), // placeholder until tick arrays
                cu_estimate: 50_000,
                state: Some(edge_delta::State::Clmm(ClmmState {
                    sqrt_price_x64: sqrt_price_x64.to_le_bytes().to_vec(),
                    liquidity: liquidity.to_le_bytes().to_vec(),
                    tick_current: *tick_current,
                    tick_spacing: 0, // unknown until we parse full pool or config
                    size_to_next_in: 0u128.to_le_bytes().to_vec(),
                })),
            };
            vec![state(0), state(1)]
        }
        _ => vec![],
    }
}

/// Start the live geyser subscription returning a handle + snapshot receiver
pub async fn start_live_stream(
    endpt: &str,
    x_token: &str,
) -> Result<(JoinHandle<()>, mpsc::Receiver<LivePoolSnapshot>)> {
    let (tx, rx) = mpsc::channel::<LivePoolSnapshot>(10_000);
    let endpoint = endpt.to_string();
    let token = x_token.to_string();

    let handle = tokio::spawn(async move {
        if let Err(e) = run_stream(endpoint, token, tx).await {
            error!("live_stream_terminated err={}", e);
        }
    });
    Ok((handle, rx))
}

async fn run_stream(
    endpoint: String,
    x_token: String,
    tx: mpsc::Sender<LivePoolSnapshot>,
) -> Result<()> {
    info!("live_stream_connecting endpoint={}", endpoint);
    let mut client = GeyserGrpcClient::build_from_shared(endpoint.clone())?
        .x_token(Some(x_token))?
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(10))
        .max_decoding_message_size(256 * 1024 * 1024)
        .connect()
        .await?;

    // Owner filters via env (comma separated base58 program IDs) to reduce stream load.
    let owner_filters: Vec<String> = std::env::var("GEYSER_OWNER_FILTERS")
        .ok()
        .map(|v| {
            v.split(',')
                .filter_map(|s| {
                    let t = s.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let mut accounts_map: std::collections::HashMap<String, SubscribeRequestFilterAccounts> =
        std::collections::HashMap::new();
    if !owner_filters.is_empty() {
        accounts_map.insert(
            "filtered".to_string(),
            SubscribeRequestFilterAccounts {
                account: vec![],
                owner: owner_filters.clone(),
                nonempty_txn_signature: None,
                filters: vec![],
            },
        );
        info!(
            "live_stream_owner_filters_applied owners_count={} owners={}",
            owner_filters.len(),
            owner_filters.join(";")
        );
    }
    let request = SubscribeRequest {
        accounts: accounts_map,
        commitment: Some(CommitmentLevel::Processed as i32),
        ..Default::default()
    };
    let (_sub_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;
    info!("live_stream_connected");

    // Simple counters for initial visibility + fee cache for Raydium
    let mut raydium_clmm_accounts: u64 = 0;
    let mut whirlpool_accounts: u64 = 0;
    let mut amm_config_fee_cache: HashMap<[u8; 32], (u32, u32, u32)> = HashMap::new(); // trade, protocol, fund
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(upd) => match upd.update_oneof {
                Some(UpdateOneof::Account(account)) => {
                    if let Some(inner) = account.account.as_ref() {
                        let owner_b58 = bs58::encode(&inner.owner).into_string();
                        if owner_b58 == RAYDIUM_CLMM_PROGRAM_ID {
                            raydium_clmm_accounts += 1;
                            if raydium_clmm_accounts % 500 == 0 {
                                info!(
                                    "raydium_clmm_account_updates count={}",
                                    raydium_clmm_accounts
                                );
                            }
                            // AmmConfig account?
                            if let Some((trade, protocol, fund)) =
                                decode_raydium_amm_config(&inner.data)
                            {
                                let mut key = [0u8; 32];
                                key.copy_from_slice(&inner.pubkey);
                                amm_config_fee_cache.insert(key, (trade, protocol, fund));
                                continue; // not a pool state
                            }
                            if let Some(decoded) = decode_raydium_clmm(&inner.data) {
                                let pool_pk = bs58::encode(&inner.pubkey).into_string();
                                let token_a = bs58::encode(&decoded.token_mint0).into_string();
                                let token_b = bs58::encode(&decoded.token_mint1).into_string();
                                // Look up amm_config fees
                                let mut cfg_key = [0u8; 32];
                                cfg_key.copy_from_slice(&decoded.amm_config);
                                let (trade_fee, protocol_fee, fund_fee) = amm_config_fee_cache
                                    .get(&cfg_key)
                                    .copied()
                                    .unwrap_or((0, 0, 0));
                                let total_fee_bps = trade_fee
                                    .saturating_add(protocol_fee)
                                    .saturating_add(fund_fee);
                                if raydium_clmm_accounts <= 20 {
                                    info!("raydium_clmm_snapshot pool={} fee_bps={} sqrt_price_x64={} liquidity={} tick={} token_a={} token_b={}",
                                        pool_pk, total_fee_bps, decoded.sqrt_price_x64, decoded.liquidity, decoded.tick_current, token_a, token_b);
                                }
                                let snap = LivePoolSnapshot::RaydiumClmm {
                                    pool: pool_pk,
                                    token_a,
                                    token_b,
                                    fee_bps: total_fee_bps,
                                    sqrt_price_x64: decoded.sqrt_price_x64,
                                    liquidity: decoded.liquidity,
                                    tick_current: decoded.tick_current,
                                    slot: account.slot as u64,
                                    ts_ns: current_time_ns(),
                                    decimals_a: decoded.mint_decimals_0,
                                    decimals_b: decoded.mint_decimals_1,
                                };
                                let _ = tx.try_send(snap);
                            }
                        } else if owner_b58 == ORCA_WHIRLPOOL_PROGRAM_ID {
                            whirlpool_accounts += 1;
                            if whirlpool_accounts % 500 == 0 {
                                info!("whirlpool_account_updates count={}", whirlpool_accounts);
                            }
                            if let Ok(w) = decode_whirlpool(&inner.data) {
                                let pool_pk = bs58::encode(&inner.pubkey).into_string();
                                let token_a = bs58::encode(w.token_mint_a.as_ref()).into_string();
                                let token_b = bs58::encode(w.token_mint_b.as_ref()).into_string();
                                let fee_bps = (w.fee_rate as u32) + (w.protocol_fee_rate as u32); // quick aggregate
                                if whirlpool_accounts <= 20 {
                                    info!("orca_whirlpool_snapshot pool={} fee_bps={} sqrt_price_x64={} liquidity={} tick={} token_a={} token_b={}",
                                        pool_pk, fee_bps, w.sqrt_price, w.liquidity, w.tick_current_index, token_a, token_b);
                                }
                                let snap = LivePoolSnapshot::OrcaWhirlpool {
                                    pool: pool_pk,
                                    token_a,
                                    token_b,
                                    fee_bps,
                                    sqrt_price_x64: w.sqrt_price,
                                    liquidity: w.liquidity,
                                    tick_current: w.tick_current_index,
                                    tick_spacing: w.tick_spacing as u16,
                                    slot: account.slot as u64,
                                    ts_ns: current_time_ns(),
                                    // Decimals not present in account; leave 0 (to be hydrated later)
                                    decimals_a: 0,
                                    decimals_b: 0,
                                };
                                let _ = tx.try_send(snap);
                            }
                        }
                    }
                }
                _ => {}
            },
            Err(e) => {
                warn!("live_stream_error err={}", e);
                break;
            }
        }
    }
    info!(
        "live_stream_ended raydium_clmm_updates={} whirlpool_updates={}",
        raydium_clmm_accounts, whirlpool_accounts
    );
    Ok(())
}

#[inline]
fn current_time_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
