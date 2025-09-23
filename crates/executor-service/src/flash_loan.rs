use anyhow::{anyhow, bail, Context, Result};
use dashmap::DashMap;
use flash_loan_providers::kamino::KaminoProvider;
use flash_loan_providers::marginfi::MarginfiProvider;
use flash_loan_providers::port_finance::PortFinanceProvider;
use flash_loan_providers::solend::SolendProvider;
use flash_loan_providers::{
    get_provider, providers as registered_providers, FlashLoanEnvironment,
    FlashLoanProvider as PluginProvider, PlanRequest as ProviderPlanRequest,
    ProviderId as PluginProviderId, QuoteRequest as ProviderQuoteRequest,
    QuoteResponse as ProviderQuoteResponse,
};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::config::ExecutorConfig;
use crate::metrics::MetricsServer;
use crate::shyft_client::ShyftClient;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FlashLoanProvider {
    Solend,
    Marginfi,
    Tulip,
    Mango,
    PortFinance,
    Kamino,
}

impl FlashLoanProvider {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "solend" | "save" => Some(Self::Solend),
            "marginfi" => Some(Self::Marginfi),
            "tulip" => Some(Self::Tulip),
            "mango" => Some(Self::Mango),
            "portfinance" | "port" => Some(Self::PortFinance),
            "kamino" | "kamino lend" | "k-lend" => Some(Self::Kamino),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Solend => "solend",
            Self::Marginfi => "marginfi",
            Self::Tulip => "tulip",
            Self::Mango => "mango",
            Self::PortFinance => "portfinance",
            Self::Kamino => "kamino",
        }
    }

    pub fn program_id(&self) -> &'static str {
        match self {
            Self::Solend => "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo",
            Self::Marginfi => "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA",
            Self::Tulip => "4bcFeLv4nydFrsZqV5CgwCVrPhkQKsXtzfy2KyMz7ozM",
            Self::Mango => "mv3ekLzLbnVPNxjSKvqBpU3ZeZXPQdEC3bp5MDEBG68",
            Self::PortFinance => "Port7uDYB1NMso4t9LZgq4u7GsAmYHj3jdkd8fHRh",
            Self::Kamino => "KMino8LendZzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanQuote {
    pub provider: FlashLoanProvider,
    pub token_mint: String,
    pub available_amount: u64,
    pub fee_bps: u16,
    pub fee_lamports: u64,
    pub estimated_latency_ms: u16,
    pub confidence: f32,
    pub is_available: bool,
    pub quote_timestamp: u64,
}

#[derive(Clone)]
struct RegisteredProvider {
    label: FlashLoanProvider,
    handler: Arc<dyn PluginProvider>,
}

#[derive(Debug, Clone)]
struct ProviderHealth {
    is_healthy: bool,
}

pub struct FlashLoanAggregator {
    _shyft_client: Arc<ShyftClient>,
    environment: FlashLoanEnvironment,
    providers: Vec<RegisteredProvider>,
    quote_cache: Arc<DashMap<String, FlashLoanQuote>>, // token_mint -> best quote
    provider_health: Arc<DashMap<FlashLoanProvider, ProviderHealth>>,
    last_quote_refresh: AtomicU64,
    metrics: Option<Arc<MetricsServer>>,
}

#[derive(Debug, Clone, Default)]
pub struct FlashLoanPlanOptions {
    pub receiver_program_id: Option<Pubkey>,
    pub receiver_payload: Option<Vec<u8>>,
    pub reserve_hint: Option<Pubkey>,
}

impl FlashLoanAggregator {
    pub fn new(
        config: Arc<ExecutorConfig>,
        shyft_client: Arc<ShyftClient>,
        metrics: Option<Arc<MetricsServer>>,
    ) -> Result<Self> {
        // Ensure built-in providers are registered before discovery
        MarginfiProvider::register();
        SolendProvider::register();
        PortFinanceProvider::register();
        KaminoProvider::register();
        let _ = registered_providers(); // initialize registry lazily

        let requested: Vec<String> = config
            .flash_loan_providers
            .iter()
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .collect();

        let mut providers = Vec::new();
        for name in requested {
            if let Some(kind) = FlashLoanProvider::from_string(&name) {
                let provider_id = PluginProviderId::from(name.as_str());
                match get_provider(&provider_id) {
                    Some(handler) => {
                        providers.push(RegisteredProvider {
                            label: kind,
                            handler,
                        });
                    }
                    None => {
                        warn!("Configured flash loan provider '{}' not registered", name);
                    }
                }
            } else {
                warn!("Unknown flash loan provider '{}' in configuration", name);
            }
        }

        if providers.is_empty() {
            bail!("No flash loan providers available after registration");
        }

        info!(
            "Flash loan aggregator initialized with {} providers",
            providers.len()
        );
        for provider in &providers {
            info!("  - {}", provider.label.as_str());
        }

        let aggregator = Self {
            _shyft_client: shyft_client,
            environment: FlashLoanEnvironment::default(),
            providers,
            quote_cache: Arc::new(DashMap::new()),
            provider_health: Arc::new(DashMap::new()),
            last_quote_refresh: AtomicU64::new(0),
            metrics,
        };

        // Seed health information
        for provider in &aggregator.providers {
            aggregator
                .provider_health
                .insert(provider.label, ProviderHealth { is_healthy: true });
        }

        Ok(aggregator)
    }

    #[allow(dead_code)]
    pub fn providers(&self) -> Vec<FlashLoanProvider> {
        self.providers.iter().map(|p| p.label).collect()
    }

    pub async fn get_best_quote(&self, token_mint: &str, amount: u64) -> Result<FlashLoanQuote> {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let last_refresh = self.last_quote_refresh.load(Ordering::Relaxed);
        let should_refresh = now.saturating_sub(last_refresh) > 5_000;

        if should_refresh {
            let quotes = self.get_all_quotes(token_mint, amount).await;
            if let Some(best) = quotes.first() {
                self.quote_cache
                    .insert(token_mint.to_string(), best.clone());
            }
            self.last_quote_refresh.store(now, Ordering::Relaxed);
        }

        self.quote_cache
            .get(token_mint)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| anyhow!("No flash loan quotes available for {token_mint}"))
    }

    pub async fn get_all_quotes(&self, token_mint: &str, amount: u64) -> Vec<FlashLoanQuote> {
        let mut quotes = Vec::new();
        let Ok(mint_pubkey) = Pubkey::from_str(token_mint) else {
            warn!("Invalid token mint for flash loan quotes: {}", token_mint);
            return quotes;
        };

        let mut tasks = Vec::new();
        let metrics = self.metrics.clone();
        for provider in &self.providers {
            if self.is_provider_healthy(provider.label) {
                let env = self.environment.clone();
                let handler = Arc::clone(&provider.handler);
                let label = provider.label;
                let request = ProviderQuoteRequest {
                    token_mint: mint_pubkey,
                    amount,
                    reserve_hint: None,
                };
                let metrics = metrics.clone();
                tasks.push(tokio::spawn(async move {
                    if let Some(ref metrics) = metrics {
                        metrics.record_flash_loan_quote_attempt();
                    }
                    match handler.quote(&env, &request).await {
                        Ok(Some(response)) => Some(convert_quote(label, response)),
                        Ok(None) => None,
                        Err(err) => {
                            warn!("Quote error from provider {}: {}", label.as_str(), err);
                            None
                        }
                    }
                }));
            }
        }

        for task in tasks {
            if let Ok(Some(quote)) = task.await {
                quotes.push(quote);
            }
        }

        quotes.sort_by_key(|q| q.fee_lamports);
        quotes
    }

    #[allow(dead_code)]
    pub async fn build_plan(
        &self,
        provider: FlashLoanProvider,
        token_mint: &str,
        amount: u64,
        borrower_pubkey: &str,
        options: FlashLoanPlanOptions,
    ) -> Result<flash_loan_providers::PlanResponse> {
        let registered = self
            .providers
            .iter()
            .find(|p| p.label == provider)
            .context("Requested flash loan provider is not registered")?;

        let borrower = Pubkey::from_str(borrower_pubkey)
            .context("Invalid borrower pubkey for flash loan plan")?;
        let token_pubkey =
            Pubkey::from_str(token_mint).context("Invalid token mint for flash loan plan")?;

        let result = registered
            .handler
            .build_plan(
                &self.environment,
                &ProviderPlanRequest {
                    token_mint: token_pubkey,
                    amount,
                    borrower,
                    receiver_program_id: options.receiver_program_id,
                    receiver_payload: options.receiver_payload,
                    reserve_hint: options.reserve_hint,
                },
            )
            .await;

        if let Some(metrics) = &self.metrics {
            match &result {
                Ok(plan) => metrics.record_flash_loan_plan_success(plan.fee_lamports),
                Err(_) => metrics.record_flash_loan_plan_failure(),
            }
        }

        result
    }

    fn is_provider_healthy(&self, provider: FlashLoanProvider) -> bool {
        self.provider_health
            .get(&provider)
            .map(|health| health.is_healthy)
            .unwrap_or(false)
    }
}

fn convert_quote(provider: FlashLoanProvider, response: ProviderQuoteResponse) -> FlashLoanQuote {
    FlashLoanQuote {
        provider,
        token_mint: response.token_mint.to_string(),
        available_amount: response.available_amount,
        fee_bps: response.fee_bps,
        fee_lamports: response.fee_lamports,
        estimated_latency_ms: response.estimated_latency_ms,
        confidence: response.confidence,
        is_available: true,
        quote_timestamp: chrono::Utc::now().timestamp_millis() as u64,
    }
}
