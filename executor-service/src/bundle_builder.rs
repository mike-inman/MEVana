use anyhow::Result;
use dashmap::DashMap;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration, Instant};

use crate::execution_engine::ArbitrageOpportunity;
use crate::flash_loan::FlashLoanProvider;
use crate::metrics::MetricsServer;
use crate::shyft_client::ShyftClient;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageBundle {
    pub bundle_id: String,
    pub transaction_ids: Vec<String>,
    pub estimated_cu: u32,
    pub estimated_fees_lamports: u64,
    pub instructions_count: usize,
}

/// Builds Jito bundles for arbitrage execution
pub struct BundleBuilder {
    shyft_client: Arc<ShyftClient>,
    route_templates: Arc<DashMap<RouteTemplateKey, Arc<RouteTemplate>>>,
    blockhash_state: Arc<BlockhashManager>,
    metrics: Option<Arc<MetricsServer>>, // optional until wired
    default_keypair: Arc<Keypair>,       // temporary signing key (placeholder)
    account_registry: Option<Arc<crate::account_registry::AccountRegistry>>, // for real metas
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RouteTemplateKey {
    pub hop_count: u8,
    pub flash_loan_provider: u8,
    pub venues_bitset: u32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RouteTemplate {
    pub key: RouteTemplateKey,
    pub static_instruction_skeletons: Vec<InstructionSkeleton>,
    pub dynamic_patches: Vec<DynamicPatch>,
    pub base_cu_estimate: u32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InstructionSkeleton {
    pub program_id: String,
    pub account_metas: Vec<AccountMetaLite>,
    pub data_base64: String, // with zeroed dynamic fields
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AccountMetaLite {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Debug, Clone)]
pub struct DynamicPatch {
    pub instruction_index: usize,
    pub offset: usize,
    pub len: usize,
    pub field: DynamicField,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DynamicField {
    AmountIn,
    MinOut,
    FlashLoanAmount,
    RepayAmount,
    PriorityFeeLamports,
}

/// Supported venue program registry (extend as needed)
#[derive(Debug, Clone, Copy)]
pub enum VenueProgram {
    RaydiumClmm,
    OrcaWhirlpool,
    MeteoraDlmm,
    Phoenix,
    OpenBook,
    Unknown,
}

impl VenueProgram {
    pub fn program_id(&self) -> &'static str {
        match self {
            VenueProgram::RaydiumClmm => "CAMMCzo5YLh62Dhptj8iE5V6VsuofTgFVgbhA7ULvP8K", // Raydium CLMM
            VenueProgram::OrcaWhirlpool => "whirLbMiicVdio4qvUfM5KAg6n2L9Jaj4woGC2YjX5J", // Orca Whirlpool (example)
            VenueProgram::MeteoraDlmm => "DLMM111111111111111111111111111111111111111", // placeholder
            VenueProgram::Phoenix => "PhoeNiXHByh2GzWxZmfZ1vZVQZ1d2n8S6tZqYkG7DNK", // Phoenix DEX
            VenueProgram::OpenBook => "openBk99xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", // placeholder
            VenueProgram::Unknown => "11111111111111111111111111111111",
        }
    }
}

fn map_venue_id(id: u32) -> VenueProgram {
    match id {
        0 => VenueProgram::RaydiumClmm,
        1 => VenueProgram::OrcaWhirlpool,
        2 => VenueProgram::MeteoraDlmm,
        3 => VenueProgram::Phoenix,
        4 => VenueProgram::OpenBook,
        _ => VenueProgram::Unknown,
    }
}

fn flash_loan_provider_to_u8(p: FlashLoanProvider) -> u8 {
    match p {
        FlashLoanProvider::Solend => 0,
        FlashLoanProvider::Marginfi => 1,
        FlashLoanProvider::Tulip => 2,
        FlashLoanProvider::Mango => 3,
        FlashLoanProvider::PortFinance => 4,
        FlashLoanProvider::JetProtocol => 5,
    }
}

#[derive(Debug)]
pub struct BlockhashManager {
    inner: RwLock<BlockhashState>,
}

#[derive(Debug, Clone)]
struct BlockhashState {
    pub current: String,
    pub last_updated: Instant,
}

impl BlockhashManager {
    pub fn new(initial: String) -> Self {
        Self {
            inner: RwLock::new(BlockhashState {
                current: initial,
                last_updated: Instant::now(),
            }),
        }
    }
    pub async fn get(&self) -> String {
        self.inner.read().await.current.clone()
    }
    pub async fn set(&self, bh: String) {
        let mut w = self.inner.write().await;
        w.current = bh;
        w.last_updated = Instant::now();
    }
}

impl BundleBuilder {
    pub fn new(shyft_client: Arc<ShyftClient>) -> Self {
        let route_templates = Arc::new(DashMap::new());
        let blockhash_state = Arc::new(BlockhashManager::new("".into()));
        let default_keypair = Arc::new(Keypair::new());
        Self {
            shyft_client,
            route_templates,
            blockhash_state,
            metrics: None,
            default_keypair,
            account_registry: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<MetricsServer>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn with_account_registry(
        mut self,
        registry: Arc<crate::account_registry::AccountRegistry>,
    ) -> Self {
        self.account_registry = Some(registry);
        self
    }

    /// Start background refresh tasks (blockhash + template warm)
    pub fn start_background_tasks(&self) {
        // Blockhash refresher (can be disabled with BLOCKHASH_REFRESH_DISABLE=1)
        let disabled = std::env::var("BLOCKHASH_REFRESH_DISABLE")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !disabled {
            let bh_mgr = Arc::clone(&self.blockhash_state);
            let client = Arc::clone(&self.shyft_client);
            let metrics_opt = self.metrics.clone();
            tokio::spawn(async move {
                let mut tick = interval(Duration::from_millis(400));
                loop {
                    tick.tick().await;
                    match client.get_recent_blockhash().await {
                        Ok(bh) => {
                            bh_mgr.set(bh).await;
                            if let Some(m) = metrics_opt.as_ref() {
                                m.blockhash_refreshes.inc();
                            }
                        }
                        Err(e) => warn!("blockhash_refresh_error: {}", e),
                    }
                }
            });
        } else {
            info!("blockhash_refresh_disabled env=BLOCKHASH_REFRESH_DISABLE");
        }
    }

    /// Build a complete arbitrage bundle from an opportunity
    pub async fn build_arbitrage_bundle(
        &self,
        opportunity: &ArbitrageOpportunity,
        flash_loan_provider: Option<FlashLoanProvider>,
    ) -> Result<ArbitrageBundle> {
        info!(
            "Building arbitrage bundle for opportunity {}",
            opportunity.opportunity_id
        );

        // Use cached blockhash (fallback to fresh if empty)
        let mut recent_blockhash = self.blockhash_state.get().await;
        if recent_blockhash.is_empty() {
            recent_blockhash = self.shyft_client.get_recent_blockhash().await?;
            self.blockhash_state.set(recent_blockhash.clone()).await;
        }

        // Attempt to find or create a route template
        let venues_bitset = Self::compute_venues_bitset(&opportunity.venues);
        let provider_u8 = flash_loan_provider
            .map(flash_loan_provider_to_u8)
            .unwrap_or(255); // 255=unknown
        let template_key = RouteTemplateKey {
            hop_count: opportunity.edges.len() as u8,
            flash_loan_provider: provider_u8,
            venues_bitset,
        };
        let template = match self.route_templates.get(&template_key) {
            Some(t) => {
                if let Some(m) = &self.metrics {
                    m.route_template_hits.inc();
                }
                t.clone()
            }
            None => {
                if let Some(m) = &self.metrics {
                    m.route_template_misses.inc();
                }
                let new_t = Arc::new(
                    self.build_route_template(&template_key, opportunity, flash_loan_provider)
                        .await?,
                );
                self.route_templates
                    .insert(template_key.clone(), new_t.clone());
                new_t
            }
        };
        // Materialize dynamic instructions from template
        let dynamic_values = self.compute_dynamic_values(opportunity).await?;
        let all_instructions = self.instantiate_from_template(&template, &dynamic_values)?;
        if let Some(m) = &self.metrics {
            m.record_instantiation(all_instructions.len(), 0);
        }

        // Build transactions (group instructions to fit in transaction limits)
        let instructions_count = all_instructions.len();
        let transactions = self
            .build_transactions(all_instructions, &recent_blockhash)
            .await?;

        let bundle = ArbitrageBundle {
            bundle_id: uuid::Uuid::new_v4().to_string(),
            transaction_ids: transactions,
            estimated_cu: opportunity.total_cu_estimate,
            estimated_fees_lamports: self.estimate_bundle_fees(opportunity).await?,
            instructions_count,
        };

        info!(
            "Built bundle {} with {} transactions",
            bundle.bundle_id,
            bundle.transaction_ids.len()
        );

        Ok(bundle)
    }

    async fn build_transactions(
        &self,
        instructions: Vec<String>,
        blockhash: &str,
    ) -> Result<Vec<String>> {
        let recent_hash = Hash::from_str(blockhash).unwrap_or_default();
        let payer = self.default_keypair.pubkey();
        let mut txs_b64 = Vec::new();
        for chunk in instructions.chunks(8) {
            let mut ix_vec: Vec<Instruction> = Vec::new();
            for encoded in chunk {
                if let Ok(raw) = BASE64_STANDARD.decode(encoded) {
                    // Derive pseudo program id from first byte for now
                    let tag = raw.get(1).cloned().unwrap_or_default();
                    // Use fixed-length 12-byte labels to keep array sizing uniform
                    let label: &[u8; 12] = match tag {
                        1 => b"flash_loan__", // 12 bytes
                        2 => b"dex_________", // 12 bytes
                        3 => b"flash_loan__",
                        _ => b"generic_____", // 12 bytes
                    };
                    let pid = Pubkey::new_from_array(hash_to_32(label));
                    let data = if raw.len() > 64 {
                        raw[..64].to_vec()
                    } else {
                        raw.clone()
                    };
                    ix_vec.push(Instruction {
                        program_id: pid,
                        accounts: vec![],
                        data,
                    });
                }
            }
            if ix_vec.is_empty() {
                continue;
            }
            let message = Message::new(&ix_vec, Some(&payer));
            let mut tx = Transaction::new_unsigned(message);
            tx.try_partial_sign(&[&*self.default_keypair], recent_hash)?;
            let serialized = bincode::serialize(&tx).unwrap_or_default();
            txs_b64.push(BASE64_STANDARD.encode(serialized));
        }
        debug!("Created {} packed transactions", txs_b64.len());
        Ok(txs_b64)
    }

    async fn estimate_bundle_fees(&self, opportunity: &ArbitrageOpportunity) -> Result<u64> {
        // Estimate total fees for the bundle
        let base_fee_per_tx = 5000; // 0.000005 SOL
        let priority_fee = 10000; // 0.00001 SOL priority fee
        let num_transactions = opportunity.edges.len() + 2; // swaps + borrow + repay

        let total_fees = (base_fee_per_tx + priority_fee) * num_transactions as u64;

        debug!("Estimated bundle fees: {} lamports", total_fees);
        Ok(total_fees)
    }

    /// Simulate bundle execution before submission (RPC based)
    pub async fn simulate_bundle(
        &self,
        bundle: &ArbitrageBundle,
    ) -> Result<BundleSimulationResult> {
        info!(
            "Simulating bundle {} ({} txs)",
            bundle.bundle_id,
            bundle.transaction_ids.len()
        );
        let started = Instant::now();
        let mut logs = Vec::new();
        let mut total_cu: u32 = 0;
        let mut success = true;
        let mut token_deltas: std::collections::HashMap<String, i128> =
            std::collections::HashMap::new();
        for (i, tx_b64) in bundle.transaction_ids.iter().enumerate() {
            match self.shyft_client.simulate_transaction(tx_b64).await {
                Ok(resp) => {
                    logs.push(format!("tx[{}] ok", i));
                    if let Some(cu) = resp["value"]["unitsConsumed"].as_u64() {
                        total_cu += cu as u32;
                    }
                    // Extract token balance diffs if present
                    if let (Some(pre), Some(post)) = (
                        resp["value"]["preTokenBalances"].as_array(),
                        resp["value"]["postTokenBalances"].as_array(),
                    ) {
                        use std::collections::HashMap as StdHashMap;
                        let mut pre_map: StdHashMap<(String, String), i128> = StdHashMap::new();
                        for entry in pre.iter() {
                            if let (Some(mint), Some(owner), Some(amount_str)) = (
                                entry["mint"].as_str(),
                                entry["owner"].as_str(),
                                entry["uiTokenAmount"]["amount"].as_str(),
                            ) {
                                if let Ok(val) = amount_str.parse::<i128>() {
                                    pre_map.insert((mint.to_string(), owner.to_string()), val);
                                }
                            }
                        }
                        for entry in post.iter() {
                            if let (Some(mint), Some(owner), Some(amount_str)) = (
                                entry["mint"].as_str(),
                                entry["owner"].as_str(),
                                entry["uiTokenAmount"]["amount"].as_str(),
                            ) {
                                if let Ok(post_val) = amount_str.parse::<i128>() {
                                    let key = (mint.to_string(), owner.to_string());
                                    let pre_val = pre_map.get(&key).cloned().unwrap_or(0);
                                    let delta = post_val - pre_val;
                                    if delta != 0 {
                                        *token_deltas.entry(mint.to_string()).or_insert(0) += delta;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    logs.push(format!("tx[{}] error: {}", i, e));
                    success = false;
                    break;
                }
            }
        }
        // Emit TOKEN_DELTA lines (summed across txs)
        for (mint, delta) in token_deltas.iter() {
            logs.push(format!("TOKEN_DELTA {} {}", mint, delta));
        }
        let result = BundleSimulationResult {
            success,
            estimated_profit_lamports: 0, // placeholder until PnL calc
            estimated_cu_consumed: if total_cu == 0 {
                bundle.estimated_cu
            } else {
                total_cu
            },
            error_message: if success {
                None
            } else {
                Some("simulation_failed".into())
            },
            simulation_logs: logs,
        };
        if let Some(m) = &self.metrics {
            m.record_simulation(result.success, started.elapsed().as_secs_f64());
        }
        Ok(result)
    }
}
impl BundleBuilder {
    fn compute_venues_bitset(venues: &[u32]) -> u32 {
        let mut bits = 0u32;
        for &v in venues {
            if v < 32 {
                bits |= 1u32 << v;
            }
        }
        bits
    }

    async fn build_route_template(
        &self,
        key: &RouteTemplateKey,
        opportunity: &ArbitrageOpportunity,
        flash_loan_provider: Option<FlashLoanProvider>,
    ) -> Result<RouteTemplate> {
        let build_started = Instant::now();
        let mut static_instruction_skeletons = Vec::new();
        let mut dynamic_patches = Vec::new();
        let _ = opportunity; // currently only used for hop_count/venues; suppress unused warnings in certain feature combos

        // Borrow (FlashLoanAmount, PriorityFeeLamports)
        let (borrow_bytes, borrow_map) = build_bytes(
            OP_FLASH_BORROW,
            &[
                DynamicField::FlashLoanAmount,
                DynamicField::PriorityFeeLamports,
            ],
        );
        let borrow_base64 = BASE64_STANDARD.encode(&borrow_bytes);
        for (field, slot_idx) in borrow_map {
            dynamic_patches.push(DynamicPatch {
                instruction_index: 0,
                offset: 4 + slot_idx * 8,
                len: 8,
                field,
            });
        }
        let flash_prog_id = flash_loan_provider
            .map(|p| p.program_id().to_string())
            .unwrap_or_else(|| "flash_loan_prog".to_string());
        static_instruction_skeletons.push(InstructionSkeleton {
            program_id: flash_prog_id.clone(),
            account_metas: self.build_flash_loan_account_metas(flash_loan_provider),
            data_base64: borrow_base64,
        });

        // Swaps (AmountIn, MinOut)
        for i in 0..key.hop_count {
            let (swap_bytes, swap_map) =
                build_bytes(OP_SWAP, &[DynamicField::AmountIn, DynamicField::MinOut]);
            let swap_base64 = BASE64_STANDARD.encode(&swap_bytes);
            let instr_index = 1 + i as usize;
            for (field, slot_idx) in swap_map {
                dynamic_patches.push(DynamicPatch {
                    instruction_index: instr_index,
                    offset: 4 + slot_idx * 8,
                    len: 8,
                    field,
                });
            }
            let venue_program = opportunity
                .venues
                .get(i as usize)
                .map(|v| map_venue_id(*v))
                .unwrap_or(VenueProgram::Unknown);
            static_instruction_skeletons.push(InstructionSkeleton {
                program_id: venue_program.program_id().to_string(),
                account_metas: self.build_swap_account_metas_for_hop(
                    venue_program,
                    i as usize,
                    i as usize,
                ),
                data_base64: swap_base64,
            });
        }

        // Repay (RepayAmount)
        let (repay_bytes, repay_map) = build_bytes(OP_FLASH_REPAY, &[DynamicField::RepayAmount]);
        let repay_base64 = BASE64_STANDARD.encode(&repay_bytes);
        let repay_index = 1 + key.hop_count as usize;
        for (field, slot_idx) in repay_map {
            dynamic_patches.push(DynamicPatch {
                instruction_index: repay_index,
                offset: 4 + slot_idx * 8,
                len: 8,
                field,
            });
        }
        static_instruction_skeletons.push(InstructionSkeleton {
            program_id: flash_prog_id,
            account_metas: self.build_flash_loan_account_metas(flash_loan_provider),
            data_base64: repay_base64,
        });

        debug!(
            "route_template_build hop_count={} dyn_patches={}",
            key.hop_count,
            dynamic_patches.len()
        );
        if let Some(m) = &self.metrics {
            m.observe_route_template_build_time(build_started.elapsed().as_secs_f64());
        }
        Ok(RouteTemplate {
            key: key.clone(),
            static_instruction_skeletons,
            dynamic_patches,
            base_cu_estimate: opportunity.total_cu_estimate,
        })
    }

    async fn compute_dynamic_values(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<HashMap<DynamicField, u128>> {
        let mut m = HashMap::new();
        m.insert(DynamicField::FlashLoanAmount, opportunity.max_size_in);
        m.insert(DynamicField::AmountIn, opportunity.max_size_in);
        let slippage_bps = 50u128.min(opportunity.profit_bps as u128);
        let min_out = opportunity.max_size_in * (10_000 - slippage_bps) / 10_000;
        m.insert(DynamicField::MinOut, min_out);
        let priority_fee = 5_000u128 + (opportunity.profit_bps as u128 * 20);
        m.insert(DynamicField::PriorityFeeLamports, priority_fee.min(50_000));
        let repay = opportunity.max_size_in + (opportunity.max_size_in * 30 / 10_000);
        m.insert(DynamicField::RepayAmount, repay);
        Ok(m)
    }

    fn instantiate_from_template(
        &self,
        template: &RouteTemplate,
        values: &HashMap<DynamicField, u128>,
    ) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for (idx, skel) in template.static_instruction_skeletons.iter().enumerate() {
            let mut data = BASE64_STANDARD.decode(&skel.data_base64)?;
            for patch in template
                .dynamic_patches
                .iter()
                .filter(|p| p.instruction_index == idx)
            {
                if let Some(val) = values.get(&patch.field) {
                    let bytes = (*val as u64).to_le_bytes();
                    if patch.offset + patch.len <= data.len() && patch.len == 8 {
                        data[patch.offset..patch.offset + 8].copy_from_slice(&bytes);
                    }
                }
            }
            out.push(BASE64_STANDARD.encode(data));
        }
        Ok(out)
    }

    fn build_flash_loan_account_metas(
        &self,
        provider: Option<FlashLoanProvider>,
    ) -> Vec<AccountMetaLite> {
        fn placeholder(label: &str) -> String {
            format!("{label}_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
        }
        let mut metas = Vec::new();
        if let Some(p) = provider {
            metas.push(AccountMetaLite {
                pubkey: p.program_id().to_string(),
                is_signer: false,
                is_writable: false,
            });
        }
        // Attempt registry resolution
        if let (Some(p), Some(reg)) = (provider, self.account_registry.as_ref()) {
            let prov_key: &str = match p {
                FlashLoanProvider::Solend => "solend",
                FlashLoanProvider::Marginfi => "marginfi",
                FlashLoanProvider::Tulip => "tulip",
                FlashLoanProvider::Mango => "mango",
                FlashLoanProvider::PortFinance => "portfinance",
                FlashLoanProvider::JetProtocol => "jetprotocol",
            };
            if let Some(mints) = reg.data.flash_loans.get(prov_key) {
                if let Some((_, accs)) = mints.iter().next() {
                    // first available mint
                    metas.push(AccountMetaLite {
                        pubkey: accs.reserve.clone(),
                        is_signer: false,
                        is_writable: true,
                    });
                    metas.push(AccountMetaLite {
                        pubkey: accs.liquidity_supply.clone(),
                        is_signer: false,
                        is_writable: true,
                    });
                    metas.push(AccountMetaLite {
                        pubkey: accs.fee_receiver.clone(),
                        is_signer: false,
                        is_writable: true,
                    });
                    metas.push(AccountMetaLite {
                        pubkey: accs.oracle.clone(),
                        is_signer: false,
                        is_writable: false,
                    });
                } else {
                    // placeholders if empty map
                    metas.push(AccountMetaLite {
                        pubkey: placeholder("reserve"),
                        is_signer: false,
                        is_writable: true,
                    });
                    metas.push(AccountMetaLite {
                        pubkey: placeholder("liquidity_supply"),
                        is_signer: false,
                        is_writable: true,
                    });
                    metas.push(AccountMetaLite {
                        pubkey: placeholder("fee_receiver"),
                        is_signer: false,
                        is_writable: true,
                    });
                    metas.push(AccountMetaLite {
                        pubkey: placeholder("oracle"),
                        is_signer: false,
                        is_writable: false,
                    });
                }
            } else {
                // provider not present
                metas.push(AccountMetaLite {
                    pubkey: placeholder("reserve"),
                    is_signer: false,
                    is_writable: true,
                });
                metas.push(AccountMetaLite {
                    pubkey: placeholder("liquidity_supply"),
                    is_signer: false,
                    is_writable: true,
                });
                metas.push(AccountMetaLite {
                    pubkey: placeholder("fee_receiver"),
                    is_signer: false,
                    is_writable: true,
                });
                metas.push(AccountMetaLite {
                    pubkey: placeholder("oracle"),
                    is_signer: false,
                    is_writable: false,
                });
            }
        } else {
            // no provider or no registry
            metas.push(AccountMetaLite {
                pubkey: placeholder("reserve"),
                is_signer: false,
                is_writable: true,
            });
            metas.push(AccountMetaLite {
                pubkey: placeholder("liquidity_supply"),
                is_signer: false,
                is_writable: true,
            });
            metas.push(AccountMetaLite {
                pubkey: placeholder("fee_receiver"),
                is_signer: false,
                is_writable: true,
            });
            metas.push(AccountMetaLite {
                pubkey: placeholder("oracle"),
                is_signer: false,
                is_writable: false,
            });
        }
        metas.push(AccountMetaLite {
            pubkey: "SysvarRecentB1ockHashes11111111111111111111".into(),
            is_signer: false,
            is_writable: false,
        });
        metas
    }

    fn build_swap_account_metas_for_hop(
        &self,
        venue: VenueProgram,
        hop: usize,
        venue_index: usize,
    ) -> Vec<AccountMetaLite> {
        fn placeholder(hop: usize, label: &str) -> String {
            format!("{label}{hop}_AAAAAAAAAAAAAAAAAAAAAAAAAAAA")
        }
        let mut metas = Vec::new();
        metas.push(AccountMetaLite {
            pubkey: venue.program_id().to_string(),
            is_signer: false,
            is_writable: false,
        });
        if let Some(reg) = self.account_registry.as_ref() {
            match venue {
                VenueProgram::RaydiumClmm => {
                    if !reg.data.clmm_pools.is_empty() {
                        let idx = venue_index % reg.data.clmm_pools.len();
                        if let Some(pool) = reg.data.clmm_pools.values().nth(idx) {
                            metas.push(AccountMetaLite {
                                pubkey: pool.pool.clone(),
                                is_signer: false,
                                is_writable: true,
                            });
                            metas.push(AccountMetaLite {
                                pubkey: pool.token_a_vault.clone(),
                                is_signer: false,
                                is_writable: true,
                            });
                            metas.push(AccountMetaLite {
                                pubkey: pool.token_b_vault.clone(),
                                is_signer: false,
                                is_writable: true,
                            });
                            if let Some(first_tick) = pool.tick_arrays.first() {
                                metas.push(AccountMetaLite {
                                    pubkey: first_tick.clone(),
                                    is_signer: false,
                                    is_writable: false,
                                });
                            }
                            if let Some(m) = &self.metrics {
                                m.record_account_meta_resolution(metas.len(), 0);
                            }
                            return metas;
                        }
                    }
                }
                _ => {}
            }
        }
        // Fallback placeholders
        metas.push(AccountMetaLite {
            pubkey: placeholder(hop, "Pool"),
            is_signer: false,
            is_writable: true,
        });
        metas.push(AccountMetaLite {
            pubkey: placeholder(hop, "VaultA"),
            is_signer: false,
            is_writable: true,
        });
        metas.push(AccountMetaLite {
            pubkey: placeholder(hop, "VaultB"),
            is_signer: false,
            is_writable: true,
        });
        if let Some(m) = &self.metrics {
            m.record_account_meta_resolution(1, metas.len() - 1);
        }
        metas
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSimulationResult {
    pub success: bool,
    pub estimated_profit_lamports: u64,
    pub estimated_cu_consumed: u32,
    pub error_message: Option<String>,
    pub simulation_logs: Vec<String>,
}

// --- Binary instruction template helpers -------------------------------------------------------
const INSTR_VERSION: u8 = 1;
const OP_FLASH_BORROW: u8 = 1;
const OP_SWAP: u8 = 2;
const OP_FLASH_REPAY: u8 = 3;

#[allow(clippy::vec_init_then_push)]
fn build_bytes(
    opcode: u8,
    dynamic_fields: &[DynamicField],
) -> (Vec<u8>, Vec<(DynamicField, usize)>) {
    let mut bytes = Vec::new();
    bytes.push(INSTR_VERSION);
    bytes.push(opcode);
    bytes.push(0); // reserved
    bytes.push(dynamic_fields.len() as u8);
    let mut map = Vec::new();
    for (i, f) in dynamic_fields.iter().cloned().enumerate() {
        bytes.extend_from_slice(&0u64.to_le_bytes());
        map.push((f, i));
    }
    (bytes, map)
}

fn hash_to_32(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h);
    out
}
