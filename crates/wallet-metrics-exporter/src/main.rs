use anyhow::Result;
use executor_service::shyft_client::ShyftClient;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use prometheus::{Encoder, Gauge, GaugeVec, IntGauge, Registry, TextEncoder};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use warp::Filter;

/// Jupiter price API response
#[derive(Debug, Deserialize)]
struct JupiterPriceResponse {
    data: HashMap<String, JupiterTokenPrice>,
}

#[derive(Debug, Deserialize)]
struct JupiterTokenPrice {
    id: String,
    #[serde(rename = "type")]
    price_type: String,
    price: String,
}

/// Known token mint addresses
static KNOWN_TOKENS: Lazy<HashMap<&'static str, TokenMetadata>> = Lazy::new(|| {
    let mut map = HashMap::new();

    map.insert(
        "So11111111111111111111111111111111111111112",
        TokenMetadata {
            symbol: "SOL",
            decimals: 9,
        },
    );
    map.insert(
        "EPjFWdd5AufqSSqeM2qZ3ET9z5tWd3daYFJ2Q8X9MJp",
        TokenMetadata {
            symbol: "USDC",
            decimals: 6,
        },
    );
    map.insert(
        "Es9vMFrzaCERmzvXkR7E62HEheG533CuJCXLcKn6A5k",
        TokenMetadata {
            symbol: "USDT",
            decimals: 6,
        },
    );
    map.insert(
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        TokenMetadata {
            symbol: "BONK",
            decimals: 5,
        },
    );
    map.insert(
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        TokenMetadata {
            symbol: "JUP",
            decimals: 6,
        },
    );
    map.insert(
        "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
        TokenMetadata {
            symbol: "WIF",
            decimals: 6,
        },
    );
    map.insert(
        "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL",
        TokenMetadata {
            symbol: "JTO",
            decimals: 9,
        },
    );
    map.insert(
        "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3",
        TokenMetadata {
            symbol: "PYTH",
            decimals: 6,
        },
    );
    map.insert(
        "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE",
        TokenMetadata {
            symbol: "ORCA",
            decimals: 6,
        },
    );
    map.insert(
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK3zr6zYgm6X8",
        TokenMetadata {
            symbol: "MSOL",
            decimals: 9,
        },
    );

    map
});

struct TokenMetadata {
    symbol: &'static str,
    decimals: u8,
}

/// Prometheus metrics for wallet monitoring
struct WalletMetrics {
    registry: Arc<Registry>,

    // Wallet balance metrics
    wallet_balance_sol: GaugeVec,
    wallet_token_balance: GaugeVec,
    wallet_value_usd: Gauge,

    // Price metrics
    sol_price_usd: Gauge,
    token_price_usd: GaugeVec,

    // System metrics
    last_balance_update_timestamp: IntGauge,
    balance_fetch_errors_total: IntGauge,
    price_fetch_errors_total: IntGauge,
}

impl WalletMetrics {
    fn new() -> Result<Self> {
        let registry = Arc::new(Registry::new());

        let wallet_balance_sol = GaugeVec::new(
            prometheus::Opts::new(
                "mevana_wallet_balance_sol",
                "Current SOL balance in wallet",
            ),
            &["wallet"],
        )?;
        registry.register(Box::new(wallet_balance_sol.clone()))?;

        let wallet_token_balance = GaugeVec::new(
            prometheus::Opts::new(
                "mevana_wallet_token_balance",
                "Current token balance in wallet",
            ),
            &["wallet", "token", "mint"],
        )?;
        registry.register(Box::new(wallet_token_balance.clone()))?;

        let wallet_value_usd = Gauge::new(
            "mevana_wallet_value_usd",
            "Total wallet value in USD",
        )?;
        registry.register(Box::new(wallet_value_usd.clone()))?;

        let sol_price_usd = Gauge::new(
            "mevana_sol_price_usd",
            "Current SOL price in USD",
        )?;
        registry.register(Box::new(sol_price_usd.clone()))?;

        let token_price_usd = GaugeVec::new(
            prometheus::Opts::new(
                "mevana_token_price_usd",
                "Current token price in USD",
            ),
            &["token", "mint"],
        )?;
        registry.register(Box::new(token_price_usd.clone()))?;

        let last_balance_update_timestamp = IntGauge::new(
            "mevana_last_balance_update_timestamp",
            "Unix timestamp of last successful balance update",
        )?;
        registry.register(Box::new(last_balance_update_timestamp.clone()))?;

        let balance_fetch_errors_total = IntGauge::new(
            "mevana_balance_fetch_errors_total",
            "Total number of balance fetch errors",
        )?;
        registry.register(Box::new(balance_fetch_errors_total.clone()))?;

        let price_fetch_errors_total = IntGauge::new(
            "mevana_price_fetch_errors_total",
            "Total number of price fetch errors",
        )?;
        registry.register(Box::new(price_fetch_errors_total.clone()))?;

        Ok(Self {
            registry,
            wallet_balance_sol,
            wallet_token_balance,
            wallet_value_usd,
            sol_price_usd,
            token_price_usd,
            last_balance_update_timestamp,
            balance_fetch_errors_total,
            price_fetch_errors_total,
        })
    }
}

/// Price fetcher using Jupiter aggregator API
struct PriceFetcher {
    client: Client,
}

impl PriceFetcher {
    fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { client })
    }

    /// Fetch prices for multiple tokens from Jupiter
    async fn fetch_prices(&self, mint_addresses: &[&str]) -> Result<HashMap<String, f64>> {
        if mint_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let ids = mint_addresses.join(",");
        let url = format!("https://price.jup.ag/v6/price?ids={}", ids);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Jupiter price API returned status: {}", response.status());
        }

        let price_response: JupiterPriceResponse = response.json().await?;

        let mut prices = HashMap::new();
        for (mint, price_data) in price_response.data {
            if let Ok(price) = price_data.price.parse::<f64>() {
                prices.insert(mint, price);
            }
        }

        Ok(prices)
    }
}

/// Wallet balance monitor
struct WalletMonitor {
    shyft_client: Arc<ShyftClient>,
    price_fetcher: PriceFetcher,
    metrics: Arc<WalletMetrics>,
    wallet_address: String,
    update_interval_secs: u64,
}

impl WalletMonitor {
    fn new(
        shyft_client: Arc<ShyftClient>,
        metrics: Arc<WalletMetrics>,
        wallet_address: String,
        update_interval_secs: u64,
    ) -> Result<Self> {
        let price_fetcher = PriceFetcher::new()?;

        Ok(Self {
            shyft_client,
            price_fetcher,
            metrics,
            wallet_address,
            update_interval_secs,
        })
    }

    async fn run(&self) {
        info!("Starting wallet monitor for address: {}", self.wallet_address);
        info!("Update interval: {} seconds", self.update_interval_secs);

        loop {
            if let Err(e) = self.update_metrics().await {
                error!("Error updating wallet metrics: {}", e);
                self.metrics.balance_fetch_errors_total.inc();
            }

            sleep(Duration::from_secs(self.update_interval_secs)).await;
        }
    }

    async fn update_metrics(&self) -> Result<()> {
        // Fetch wallet balances
        let balances = self.shyft_client.get_wallet_balances(&self.wallet_address).await?;

        // Update SOL balance metric
        let sol_balance = balances.sol_balance as f64 / 1_000_000_000.0;
        self.metrics.wallet_balance_sol
            .with_label_values(&["hot"])
            .set(sol_balance);

        // Collect all mints for price fetching
        let mut mints_to_fetch: Vec<&str> = vec!["So11111111111111111111111111111111111111112"]; // SOL

        // Add token mints
        for mint in balances.token_balances.keys() {
            mints_to_fetch.push(mint.as_str());
        }

        // Fetch prices
        let prices = match self.price_fetcher.fetch_prices(&mints_to_fetch).await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to fetch prices from Jupiter: {}. Using fallback.", e);
                self.metrics.price_fetch_errors_total.inc();
                HashMap::new()
            }
        };

        // Update SOL price
        let sol_mint = "So11111111111111111111111111111111111111112";
        if let Some(&sol_price) = prices.get(sol_mint) {
            self.metrics.sol_price_usd.set(sol_price);
            info!("SOL price: ${:.2}", sol_price);
        }

        // Calculate total wallet value
        let mut total_value_usd = 0.0;

        // Add SOL value
        if let Some(&sol_price) = prices.get(sol_mint) {
            total_value_usd += sol_balance * sol_price;
        }

        // Update token balances and add to total value
        for (mint, &amount_raw) in &balances.token_balances {
            if amount_raw == 0 {
                continue;
            }

            let metadata = KNOWN_TOKENS.get(mint.as_str());
            let (symbol, decimals) = metadata
                .map(|m| (m.symbol, m.decimals))
                .unwrap_or((mint.as_str(), 9));

            let amount = amount_raw as f64 / 10f64.powi(decimals as i32);

            // Update token balance metric
            self.metrics.wallet_token_balance
                .with_label_values(&["hot", symbol, mint])
                .set(amount);

            // Add to total value if we have price data
            if let Some(&token_price) = prices.get(mint.as_str()) {
                self.metrics.token_price_usd
                    .with_label_values(&[symbol, mint])
                    .set(token_price);

                total_value_usd += amount * token_price;

                info!("{}: {:.6} (${:.2} each, ${:.2} total)",
                    symbol, amount, token_price, amount * token_price);
            } else {
                info!("{}: {:.6} (price unavailable)", symbol, amount);
            }
        }

        // Update total value metric
        self.metrics.wallet_value_usd.set(total_value_usd);

        // Update timestamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        self.metrics.last_balance_update_timestamp.set(now);

        info!("Total wallet value: ${:.2}", total_value_usd);
        info!("SOL balance: {:.4} SOL", sol_balance);

        Ok(())
    }
}

async fn start_metrics_server(metrics: Arc<WalletMetrics>, port: u16) -> Result<()> {
    let registry = Arc::clone(&metrics.registry);

    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .map(move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();

            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                return warp::reply::with_status(
                    format!("Failed to encode metrics: {}", e),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                );
            }

            warp::reply::with_status(
                String::from_utf8_lossy(&buffer).to_string(),
                warp::http::StatusCode::OK,
            )
        });

    let health_route = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::with_status(
                "OK".to_string(),
                warp::http::StatusCode::OK,
            )
        });

    let routes = metrics_route.or(health_route);

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    info!("Starting metrics server on {}", addr);

    warp::serve(routes).run(addr).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    info!("Starting wallet-metrics-exporter");

    // Load configuration from environment
    let wallet_address = std::env::var("WALLET_ADDRESS")
        .expect("WALLET_ADDRESS environment variable must be set");

    let api_key = std::env::var("X_TOKEN")
        .or_else(|_| std::env::var("SHYFT_API_KEY"))
        .expect("X_TOKEN or SHYFT_API_KEY environment variable must be set");

    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .or_else(|_| std::env::var("RPC_URL"))
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let jito_url = std::env::var("JITO_API_URL")
        .unwrap_or_else(|_| "https://jito-api.jito.wtf".to_string());

    let metrics_port = std::env::var("WALLET_METRICS_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(9905);

    let update_interval_secs = std::env::var("WALLET_UPDATE_INTERVAL_SECS")
        .ok()
        .and_then(|i| i.parse::<u64>().ok())
        .unwrap_or(5);

    info!("Configuration:");
    info!("  Wallet address: {}", wallet_address);
    info!("  RPC URL: {}", rpc_url);
    info!("  Metrics port: {}", metrics_port);
    info!("  Update interval: {} seconds", update_interval_secs);

    // Initialize Shyft client
    let shyft_client = Arc::new(ShyftClient::new(api_key, rpc_url, jito_url)?);

    // Initialize metrics
    let metrics = Arc::new(WalletMetrics::new()?);

    // Create wallet monitor
    let monitor = WalletMonitor::new(
        Arc::clone(&shyft_client),
        Arc::clone(&metrics),
        wallet_address,
        update_interval_secs,
    )?;

    // Start metrics server and wallet monitor concurrently
    tokio::select! {
        result = start_metrics_server(Arc::clone(&metrics), metrics_port) => {
            error!("Metrics server stopped: {:?}", result);
            result
        }
        _ = monitor.run() => {
            error!("Wallet monitor stopped unexpectedly");
            Ok(())
        }
    }
}
