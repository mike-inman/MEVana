mod instruction_account_mapper;
mod serialization;
mod token_serializable;

// prost trait (no name clash with Solana Message)
use yellowstone_grpc_proto::prost::Message as _;

use bs58;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer}; // Producer gives .client()
use rdkafka::ClientConfig;
//use clap::Parser as ClapParser;
//use clap::Parser as ClapParser;

use serde::Deserialize;
// use rdkafka::producer::{FutureProducer, FutureRecord};

use crate::token_serializable::convert_to_serializable;
use ::solana_sdk::transaction::Result as TransactionResult;
use solana_transaction_status::Rewards;

use {
    backoff::{future::retry, ExponentialBackoff},
    clap::Parser as ClapParser,
    futures::{future::TryFutureExt, sink::SinkExt, stream::StreamExt},
    instruction_account_mapper::{AccountMetadata, Idl, InstructionAccountMapper},
    log::{error, info},
    serde::Serialize,
    serialization::{serialize_option_pubkey, serialize_pubkey},
    solana_account_decoder_client_types::token::UiTokenAmount,
    solana_sdk::{
        hash::Hash,
        instruction::{AccountMeta, CompiledInstruction, Instruction},
        message::{
            v0::{LoadedAddresses, Message, MessageAddressTableLookup},
            MessageHeader, VersionedMessage,
        },
        pubkey::Pubkey,
        signature::Signature,
        transaction::{Legacy, TransactionVersion, VersionedTransaction},
        transaction_context::TransactionReturnData,
    },
    solana_transaction_status::{
        ConfirmedTransactionWithStatusMeta, InnerInstruction, InnerInstructions, Reward,
        RewardType, TransactionStatusMeta, TransactionTokenBalance, TransactionWithStatusMeta,
        VersionedTransactionWithStatusMeta,
    },
    spl_token::instruction::TokenInstruction,
    std::{
        collections::HashMap,
        env, fs,
        str::FromStr,
        sync::Arc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
    tokio::sync::Mutex,
    tonic::transport::channel::ClientTlsConfig,
    whirlpool_interface::instructions::WhirlpoolProgramIx,
    yellowstone_grpc_client::{GeyserGrpcClient, Interceptor},
    yellowstone_grpc_proto::{
        geyser::SubscribeRequestFilterTransactions,
        prelude::{
            subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
        },
    },
};

use std::sync::atomic::{AtomicU64, Ordering};
//use std::sync::Arc;

#[derive(Default)]
struct Counters {
    received: AtomicU64,
    produced_ok: AtomicU64,
    produced_err: AtomicU64,
    pings: AtomicU64,
    reconnects: AtomicU64,
}

fn mask_token(t: &str) -> String {
    if t.len() <= 6 {
        return "******".into();
    }
    format!("{}…{}", &t[..4], &t[t.len() - 2..])
}

type TxnFilterMap = HashMap<String, SubscribeRequestFilterTransactions>;

// Deprecated constant: use PROGRAM_IDS env for universal filtering
const WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// #[derive(Debug, Clone, ClapParser)]
// #[clap(author, version, about)]
// struct Args {
//     #[clap(short, long, help = "gRPC endpoint")]
//     endpoint: String,

//     #[clap(long, help = "X-Token")]
//     x_token: String,
// }

#[derive(Debug, Clone, ClapParser)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, help = "Kafka broker")]
    broker: Option<String>,

    #[clap(short, long, help = "Kafka topic")]
    topic: Option<String>,

    #[clap(short, long, help = "gRPC endpoint")]
    endpoint: Option<String>,

    #[clap(long, help = "X-Token")]
    x_token: Option<String>,
}

impl Args {
    async fn connect(
        endpoint: &str,
        x_token: &str,
    ) -> anyhow::Result<GeyserGrpcClient<impl Interceptor>> {
        GeyserGrpcClient::build_from_shared(endpoint.to_string())?
            .x_token(Some(x_token.to_string()))?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10))
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .max_decoding_message_size(1024 * 1024 * 1024)
            .connect()
            .await
            .map_err(Into::into)
    }

    pub fn get_txn_updates(&self) -> anyhow::Result<SubscribeRequest> {
        let mut transactions: TxnFilterMap = HashMap::new();

        // PROGRAM_IDS env: comma-separated list of program ids
        let pids_env = std::env::var("PROGRAM_IDS").ok();
        let pids: Vec<String> = pids_env
            .as_deref()
            .unwrap_or(WHIRLPOOL_PROGRAM_ID)
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        transactions.insert(
            "client".to_owned(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                account_include: pids,
                account_exclude: vec![],
                account_required: vec![],
                signature: None,
            },
        );

        Ok(SubscribeRequest {
            accounts: HashMap::default(),
            slots: HashMap::default(),
            transactions,
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

fn create_kafka_producer(broker: &str) -> anyhow::Result<FutureProducer> {
    let p: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("enable.idempotence", "true")
        .set("compression.type", "lz4")
        .set("acks", "all")
        .set("linger.ms", "5")
        .set("batch.size", "131072")
        .set("debug", "broker,protocol,topic,msg")
        .set("reconnect.backoff.ms", "500")
        .set("reconnect.backoff.max.ms", "30000")
        .set("socket.keepalive.enable", "true")
        .create()
        .map_err(|e| anyhow::anyhow!("Failed to create Kafka producer: {e}"))?;
    Ok(p)
}

fn verify_kafka_connection(producer: &FutureProducer, topic: &str) -> anyhow::Result<()> {
    let md = producer
        .client()
        .fetch_metadata(Some(topic), std::time::Duration::from_secs(5))
        .map_err(|e| anyhow::anyhow!("Kafka metadata fetch failed for {topic}: {e}"))?;

    // Find the topic entry (fetch_metadata(Some(topic)) usually returns exactly this one)
    let t = md
        .topics()
        .iter()
        .find(|t| t.name() == topic)
        .ok_or_else(|| anyhow::anyhow!("Kafka reachable but topic '{topic}' not found"))?;

    if t.partitions().is_empty() {
        anyhow::bail!("Topic '{topic}' has no partitions");
    }

    let parts: Vec<String> = t
        .partitions()
        .iter()
        .map(|p| format!("p{}->leader{} (err={:?})", p.id(), p.leader(), p.error()))
        .collect();

    log::info!(
        "kafka ok: topic='{}' partitions=[{}]",
        topic,
        parts.join(", ")
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexSwapTxn {
    pub tx_signature: String,
    pub slot: u64,
    pub block_time: i64,
    pub dex: String,
    pub pool_address: Option<String>,
    pub user_wallet: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub platform_fee_bps: Option<u16>,
    pub route: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwapEvent {
    wallet: String,
    input_token: String,
    output_token: String,
    amount_in: String,
    amount_out: String,
}

#[derive(Debug, Serialize)]
struct TransactionInstructionWithParent {
    instruction: Instruction,
    parent_program_id: Option<Pubkey>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct DecodedInstruction {
    pub name: String,
    pub accounts: Vec<AccountMetadata>,
    pub data: serde_json::Value,
    #[serde(serialize_with = "serialize_pubkey")]
    pub program_id: Pubkey,
    #[serde(serialize_with = "serialize_option_pubkey")]
    pub parent_program_id: Option<Pubkey>,
}

#[derive(Debug)]
pub struct TransactionWithActions {
    pub slot: u64,
    pub tx_with_meta: TransactionWithStatusMeta,
    pub block_time: i64,
    pub actions: Vec<DecodedInstruction>,
    pub transactions: Vec<TransactionInstructionWithParent>,
    pub version: Option<TransactionVersion>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedTransaction {
    pub signatures: Vec<Signature>,
    pub message: ParsedMessage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedMessage {
    pub header: MessageHeader,
    pub account_keys: Vec<Pubkey>,
    pub recent_blockhash: Hash,
    pub instructions: Vec<DecodedInstruction>, // Replacing CompiledInstruction
    pub address_table_lookups: Vec<MessageAddressTableLookup>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedTransactionStatusMeta {
    pub status: TransactionResult<()>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub inner_instructions: Vec<DecodedInstruction>, // Replacing inner_instructions
    pub log_messages: Option<Vec<String>>,
    pub pre_token_balances: Option<Vec<TransactionTokenBalance>>,
    pub post_token_balances: Option<Vec<TransactionTokenBalance>>,
    pub rewards: Option<Rewards>,
    pub loaded_addresses: LoadedAddresses,
    pub return_data: Option<TransactionReturnData>,
    pub compute_units_consumed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedConfirmedTransaction {
    pub slot: u64,
    pub transaction: ParsedTransaction,
    pub meta: ParsedTransactionStatusMeta,
    pub block_time: Option<i64>,
}
#[derive(Debug)]
pub struct ParsedConfirmedTransactionWithStatusMeta {
    pub slot: u64,
    pub transaction: ParsedTransaction,
    pub meta: ParsedTransactionStatusMeta,
    pub block_time: Option<i64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env::set_var(
        env_logger::DEFAULT_FILTER_ENV,
        env::var_os(env_logger::DEFAULT_FILTER_ENV).unwrap_or_else(|| "info".into()),
    );
    env_logger::init();

    // 1) Parse CLI (clap) and fall back to env
    let args = Args::parse();
    let broker = args
        .broker
        .or_else(|| env::var("KAFKA_BROKER").ok())
        .expect("KAFKA_BROKER must be set");
    let topic = args
        .topic
        .or_else(|| env::var("TOPIC").ok())
        .expect("TOPIC must be set");
    let endpoint = args
        .endpoint
        .or_else(|| env::var("ENDPOINT").ok())
        .expect("ENDPOINT must be set");
    let x_token = args
        .x_token
        .or_else(|| env::var("X_TOKEN").ok())
        .expect("X_TOKEN must be set");

    log::info!(
        "config: broker={}, topic={}, endpoint={}, x_token={}",
        broker,
        topic,
        endpoint,
        mask_token(&x_token)
    );

    // 2) Build producer once and verify connectivity
    let producer = create_kafka_producer(&broker)?; // <-- use ?
    verify_kafka_connection(&producer, &topic)?; // now &FutureProducer

    let counters = Arc::new(Counters::default());

    // Stats logger every 10s
    {
        let counters = counters.clone();
        let topic = topic.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(10));
            loop {
                tick.tick().await;
                let rx = counters.received.load(Ordering::Relaxed);
                let ok = counters.produced_ok.load(Ordering::Relaxed);
                let err = counters.produced_err.load(Ordering::Relaxed);
                let ping = counters.pings.load(Ordering::Relaxed);
                let recn = counters.reconnects.load(Ordering::Relaxed);
                log::info!(
                    "stats: topic={} rx={} sent_ok={} sent_err={} pings={} reconnects={}",
                    topic,
                    rx,
                    ok,
                    err,
                    ping,
                    recn
                );
            }
        });
    }

    // Heartbeat every 15s -> <topic>.heartbeat
    {
        let hb_producer = producer.clone();
        let hb_topic = format!("{}.heartbeat", topic);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(15));
            loop {
                tick.tick().await;
                let payload = format!(
                    r#"{{"ts":{},"pid":{},"app":"dex-stream-orca"}}"#,
                    chrono::Utc::now().timestamp(),
                    std::process::id()
                );
                let _ = hb_producer
                    .send(
                        FutureRecord::to(&hb_topic).key("hb").payload(&payload),
                        Duration::from_secs(5),
                    )
                    .await;
                log::debug!("heartbeat -> {}", hb_topic);
            }
        });
    }

    // 3) Retry loop for Geyser connection
    let zero_attempts = Arc::new(Mutex::new(true));
    retry(ExponentialBackoff::default(), {
        let endpoint = endpoint.clone();
        let x_token = x_token.clone();
        let topic = topic.clone();
        let producer = producer.clone(); // clone per attempt
        let zero_attempts = Arc::clone(&zero_attempts);
        let counters = counters.clone();

        move || {
            let endpoint = endpoint.clone();
            let x_token = x_token.clone();
            let topic = topic.clone();
            let producer = producer.clone();
            let zero_attempts = Arc::clone(&zero_attempts);
            let counters = counters.clone();

            async move {
                let mut first = zero_attempts.lock().await;
                if *first {
                    *first = false;
                } else {
                    info!("Retry to connect to the server");
                }
                drop(first);

                let client = Args::connect(&endpoint, &x_token)
                    .await
                    .map_err(backoff::Error::transient)?;
                info!("Connected");

                counters.reconnects.fetch_add(1, Ordering::Relaxed);

                let mut transactions: TxnFilterMap = HashMap::new();
                transactions.insert(
                    "client".to_owned(),
                    SubscribeRequestFilterTransactions {
                        vote: Some(false),
                        failed: Some(false),
                        account_include: vec![WHIRLPOOL_PROGRAM_ID.to_string()],
                        account_exclude: vec![],
                        account_required: vec![],
                        signature: None,
                    },
                );
                let request = SubscribeRequest {
                    accounts: HashMap::default(),
                    slots: HashMap::default(),
                    transactions,
                    transactions_status: HashMap::default(),
                    blocks: HashMap::default(),
                    blocks_meta: HashMap::default(),
                    entry: HashMap::default(),
                    commitment: Some(CommitmentLevel::Processed as i32),
                    accounts_data_slice: Vec::default(),
                    ping: None,
                    from_slot: None,
                };

                geyser_subscribe(client, request, producer, topic.clone(), counters.clone())
                    .await
                    .map_err(backoff::Error::transient)?;

                Ok::<(), backoff::Error<anyhow::Error>>(())
            }
            .inspect_err(|error| error!("failed to connect: {error}"))
        }
    })
    .await?;
    Ok(()) // <— make the function type-check
}

/// Converts a string to camel case.
fn to_camel_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first_char) => first_char.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Extracts the instruction name and converts it to camel case.
fn get_instruction_name_with_typename(instruction: &TokenInstruction) -> String {
    let debug_string = format!("{:?}", instruction);
    if let Some(first_brace) = debug_string.find(" {") {
        let name = &debug_string[..first_brace]; // Extract name before `{`
        to_camel_case(name)
    } else {
        to_camel_case(&debug_string) // Directly convert unit variant names
    }
}

async fn geyser_subscribe(
    mut client: GeyserGrpcClient<impl Interceptor>,
    request: SubscribeRequest,
    producer: FutureProducer,
    topic: String,
    counters: Arc<Counters>,
) -> anyhow::Result<()> {
    let (mut subscribe_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;
    info!("stream opened");

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(update_msg) => {
                match update_msg.update_oneof {
                    Some(UpdateOneof::Transaction(update)) => {
                        counters.received.fetch_add(1, Ordering::Relaxed);

                        // 1) Key (signature base58 when present)
                        let key = update
                            .transaction
                            .as_ref()
                            .map(|txn| bs58::encode(&txn.signature).into_string())
                            .unwrap_or_else(|| format!("slot:{}", update.slot));

                        // 2) Encode entire update
                        let mut buf = Vec::new();
                        update.encode(&mut buf)?; // prost::Message

                        // 3) Produce to Kafka
                        let record = FutureRecord::to(&topic).key(&key).payload(&buf);
                        match producer.send(record, Duration::from_secs(10)).await {
                            Ok(d) => {
                                let ok = counters.produced_ok.fetch_add(1, Ordering::Relaxed) + 1;
                                // Log first few and then every 1000th
                                if ok <= 3 || ok % 1000 == 0 {
                                    log::info!(
                                        "produced: key={} partition={} offset={} ts={:?}",
                                        key,
                                        d.partition,
                                        d.offset,
                                        d.timestamp
                                    );
                                } else {
                                    log::debug!(
                                        "produced: key={} partition={} offset={} ts={:?}",
                                        key,
                                        d.partition,
                                        d.offset,
                                        d.timestamp
                                    );
                                }
                            }
                            Err((e, _msg)) => {
                                counters.produced_err.fetch_add(1, Ordering::Relaxed);
                                log::error!("kafka send failed: {e:?}");
                                tokio::time::sleep(Duration::from_millis(250)).await;
                            }
                        }

                        // If you're only forwarding raw, you can continue here
                        // continue;
                    }

                    Some(UpdateOneof::Ping(_)) => {
                        counters.pings.fetch_add(1, Ordering::Relaxed);
                        subscribe_tx
                            .send(SubscribeRequest {
                                ping: Some(SubscribeRequestPing { id: 1 }),
                                ..Default::default()
                            })
                            .await?;
                    }

                    Some(UpdateOneof::Pong(_)) => {
                        // no-op
                    }
                    Some(_) => {
                        // ignore other variants for now
                    }
                    None => {
                        error!("update not found in the message");
                        break;
                    }
                }
            }
            Err(error) => {
                error!("error: {error:?}");
                break;
            }
        }
    }

    info!("stream closed");
    Ok(())
}

fn flatten_transaction_response(
    transaction_with_meta: &VersionedTransactionWithStatusMeta,
) -> Vec<TransactionInstructionWithParent> {
    let mut result = Vec::new();

    let transaction = transaction_with_meta.transaction.clone();
    let ci_ixs = transaction.message.instructions();
    let parsed_accounts = parse_transaction_accounts(
        &transaction.message,
        transaction_with_meta.meta.loaded_addresses.clone(),
    );

    let ordered_cii = match &transaction_with_meta.meta.inner_instructions {
        Some(cii) => {
            let mut cii = cii.clone();
            cii.sort_by(|a, b| a.index.cmp(&b.index));
            cii
        }
        _ => Vec::new(),
    };

    let total_calls: usize = ordered_cii
        .iter()
        .fold(ci_ixs.len(), |acc, cii| acc + cii.instructions.len());

    let mut last_pushed_ix = -1;
    let mut call_index: isize = -1;

    for cii in ordered_cii.iter() {
        while last_pushed_ix != cii.index as i64 {
            last_pushed_ix += 1;
            call_index += 1;
            let ci_ix = &ci_ixs[last_pushed_ix as usize];
            result.push(TransactionInstructionWithParent {
                instruction: compiled_instruction_to_instruction(ci_ix, parsed_accounts.clone()),
                parent_program_id: None,
            });
        }

        for cii_entry in &cii.instructions {
            let parent_program_id =
                parsed_accounts[ci_ixs[last_pushed_ix as usize].program_id_index as usize].pubkey;

            let ix = CompiledInstruction {
                program_id_index: cii_entry.instruction.program_id_index,
                accounts: cii_entry.instruction.accounts.clone(),
                data: cii_entry.instruction.data.clone(),
            };
            result.push(TransactionInstructionWithParent {
                instruction: compiled_instruction_to_instruction(&ix, parsed_accounts.clone()),
                parent_program_id: Some(parent_program_id),
            });

            call_index += 1;
        }
    }

    while call_index < (total_calls - 1).try_into().unwrap() {
        last_pushed_ix += 1;
        call_index += 1;
        let ci_ix = &ci_ixs[last_pushed_ix as usize];
        result.push(TransactionInstructionWithParent {
            instruction: compiled_instruction_to_instruction(ci_ix, parsed_accounts.clone()),
            parent_program_id: None,
        });
    }

    result
}

fn compiled_instruction_to_instruction(
    ci: &CompiledInstruction,
    parsed_accounts: Vec<AccountMeta>,
) -> Instruction {
    let program_id = parsed_accounts[ci.program_id_index as usize].pubkey;
    let accounts: Vec<AccountMeta> = ci.accounts.iter().map(|&index| {
        if index as usize >= parsed_accounts.len() {
            panic!(
                "Trying to resolve account at index {} while parsedAccounts is only {}. \
                Looks like you're trying to parse versioned transaction, make sure that LoadedAddresses are passed to the \
                parseTransactionAccounts function",
                index, parsed_accounts.len()
            );
        }
        parsed_accounts[index as usize].clone()
    }).collect();

    Instruction {
        program_id,
        accounts,
        data: ci.data.clone(),
    }
}

fn flatten_compiled_instructions(
    transaction_with_meta: &VersionedTransactionWithStatusMeta,
) -> Vec<TransactionInstructionWithParent> {
    let mut compiled_result = Vec::new();
    let transaction = &transaction_with_meta.transaction;
    let ci_ixs = transaction.message.instructions();
    let parsed_accounts = parse_transaction_accounts(
        &transaction.message,
        transaction_with_meta.meta.loaded_addresses.clone(),
    );

    for ci_ix in ci_ixs {
        compiled_result.push(TransactionInstructionWithParent {
            instruction: compiled_instruction_to_instruction(&ci_ix, parsed_accounts.clone()),
            parent_program_id: None,
        });
    }

    compiled_result
}

fn flatten_inner_instructions(
    transaction_with_meta: &VersionedTransactionWithStatusMeta,
) -> Vec<TransactionInstructionWithParent> {
    let mut inner_result = Vec::new();
    let transaction = &transaction_with_meta.transaction;
    let ci_ixs = transaction.message.instructions();
    let parsed_accounts = parse_transaction_accounts(
        &transaction.message,
        transaction_with_meta.meta.loaded_addresses.clone(),
    );

    if let Some(inner_ixs) = &transaction_with_meta.meta.inner_instructions {
        let mut ordered_cii = inner_ixs.clone();
        ordered_cii.sort_by(|a, b| a.index.cmp(&b.index));

        for cii in ordered_cii {
            let parent_program_id =
                parsed_accounts[ci_ixs[cii.index as usize].program_id_index as usize].pubkey;

            for cii_entry in cii.instructions {
                let ix = CompiledInstruction {
                    program_id_index: cii_entry.instruction.program_id_index,
                    accounts: cii_entry.instruction.accounts.clone(),
                    data: cii_entry.instruction.data.clone(),
                };
                inner_result.push(TransactionInstructionWithParent {
                    instruction: compiled_instruction_to_instruction(&ix, parsed_accounts.clone()),
                    parent_program_id: Some(parent_program_id),
                });
            }
        }
    }

    inner_result
}

pub fn parse_transaction_accounts(
    message: &VersionedMessage,
    loaded_addresses: LoadedAddresses,
) -> Vec<AccountMeta> {
    let accounts = message.static_account_keys();
    let readonly_signed_accounts_count = message.header().num_readonly_signed_accounts as usize;
    let readonly_unsigned_accounts_count = message.header().num_readonly_unsigned_accounts as usize;
    let required_signatures_accounts_count = message.header().num_required_signatures as usize;
    let total_accounts = accounts.len();

    let mut parsed_accounts: Vec<AccountMeta> = accounts
        .iter()
        .enumerate()
        .map(|(index, pubkey)| {
            let is_writable = index
                < required_signatures_accounts_count - readonly_signed_accounts_count
                || (index >= required_signatures_accounts_count
                    && index < total_accounts - readonly_unsigned_accounts_count);

            AccountMeta {
                pubkey: *pubkey,
                is_signer: index < required_signatures_accounts_count,
                is_writable,
            }
        })
        .collect();

    parsed_accounts.extend(
        loaded_addresses
            .writable
            .into_iter()
            .map(|pubkey| AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: true,
            }),
    );

    parsed_accounts.extend(
        loaded_addresses
            .readonly
            .into_iter()
            .map(|pubkey| AccountMeta {
                pubkey,
                is_signer: false,
                is_writable: false,
            }),
    );

    parsed_accounts
}
