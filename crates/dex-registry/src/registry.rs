use anyhow::Result;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use prometheus::IntGauge;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::api::ApiServer;
use crate::cache::RegistryCache;
use crate::config::RegistryConfig;
use crate::database::Database;
use crate::discovery::DiscoveryService;
use crate::plugin::PluginManager;

/// Main DEX Registry service that coordinates all components
pub struct DexRegistry {
    config: Arc<RegistryConfig>,
    database: Arc<Database>,
    cache: Arc<RegistryCache>,
    plugin_manager: Arc<PluginManager>,
    discovery_service: Arc<DiscoveryService>,
    api_server: Arc<ApiServer>,
}

impl DexRegistry {
    pub async fn new(config: RegistryConfig) -> Result<Self> {
        let config = Arc::new(config);

        // Validate configuration
        config.validate()?;

        // Initialize database
        let database = Arc::new(
            Database::new(
                &format!("sqlite:{}", config.sqlite_path),
                config.connection_pool_size,
            )
            .await?,
        );

        // Initialize cache
        let mut cache = RegistryCache::new(&config.redis_url, config.cache_ttl_seconds).await?;
        cache.connect().await?;
        let cache = Arc::new(cache);

        // Initialize plugin manager
        let mut plugin_manager =
            PluginManager::new(config.plugin_dir.clone().into(), config.auto_load_plugins);
        plugin_manager.initialize().await?;
        let plugin_manager = Arc::new(plugin_manager);

        // Initialize discovery service
        let discovery_service = Arc::new(
            DiscoveryService::new(
                Arc::clone(&config),
                Arc::clone(&database),
                Arc::clone(&cache),
                Arc::clone(&plugin_manager),
            )
            .await?,
        );

        // Initialize API server
        let api_server = Arc::new(ApiServer::new(
            config.http_port,
            config.metrics_port,
            Arc::clone(&database),
            Arc::clone(&cache),
            Arc::clone(&plugin_manager),
        ));

        info!("DEX Registry components initialized successfully");

        Ok(Self {
            config,
            database,
            cache,
            plugin_manager,
            discovery_service,
            api_server,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting DEX Registry Service");

        // Start cache refresh task
        self.cache.start_refresh_task().await?;

        // Start discovery service
        let _discovery_handle = {
            let discovery_service = Arc::clone(&self.discovery_service);
            tokio::spawn(async move {
                if let Err(e) = discovery_service.start().await {
                    error!("Discovery service error: {}", e);
                }
            })
        };

        // Start API server
        let _api_handle = {
            let api_server = Arc::clone(&self.api_server);
            tokio::spawn(async move {
                if let Err(e) = api_server.start().await {
                    error!("API server error: {}", e);
                }
            })
        };

        // Load initial data into cache
        self.populate_initial_cache().await?;

        info!("DEX Registry service is running");
        info!("  HTTP API: http://localhost:{}", self.config.http_port);
        info!(
            "  Metrics: http://localhost:{}/metrics",
            self.config.metrics_port
        );

        // Keep main thread alive
        loop {
            sleep(Duration::from_secs(30)).await;

            // Health check
            if let Err(e) = self.health_check().await {
                error!("Health check failed: {}", e);
            }
        }
    }

    async fn populate_initial_cache(&self) -> Result<()> {
        info!("Populating initial cache with known DEXs and tokens");

        // Load DEX programs
        let dex_programs = self.database.list_dex_programs(true).await?;
        for program in dex_programs {
            if let Err(e) = self.cache.set_dex_program(&program).await {
                warn!("Failed to cache DEX program {}: {}", program.program_id, e);
            }
        }

        // Load tokens
        let tokens = self.database.list_tokens(true).await?;
        for token in tokens {
            if let Err(e) = self.cache.set_token(&token).await {
                warn!("Failed to cache token {}: {}", token.mint_address, e);
            }
        }

        let cache_stats = self.cache.get_stats();
        info!(
            "Initial cache populated: {} DEXs, {} tokens",
            cache_stats.dex_programs_cached, cache_stats.tokens_cached
        );

        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        // Check database connectivity
        let stats = self.database.get_registry_stats().await?;

        // Check cache connectivity
        let cache_stats = self.cache.get_stats();

        // Check plugin manager
        let plugin_stats = self.plugin_manager.get_stats();

        // Update Prometheus gauges
        DEX_PROGS_CACHED.set(cache_stats.dex_programs_cached as i64);
        TOKENS_CACHED.set(cache_stats.tokens_cached as i64);
        PLUGINS_TOTAL.set(plugin_stats.total_plugins as i64);
        LAST_HEALTH_CHECK.set(chrono::Utc::now().timestamp());

        // Log health status
        info!(
            "Health check: DB({} DEXs, {} tokens), Cache({} DEXs, {} tokens), Plugins({})",
            stats.active_dex_programs,
            stats.verified_tokens,
            cache_stats.dex_programs_cached,
            cache_stats.tokens_cached,
            plugin_stats.total_plugins
        );

        Ok(())
    }

    pub fn get_stats(&self) -> RegistryServiceStats {
        let cache_stats = self.cache.get_stats();
        let plugin_stats = self.plugin_manager.get_stats();

        RegistryServiceStats {
            cache_stats,
            plugin_stats,
            config_summary: ConfigSummary {
                cache_ttl_seconds: self.config.cache_ttl_seconds,
                auto_load_plugins: self.config.auto_load_plugins,
                discovery_batch_size: self.config.discovery_batch_size,
                max_concurrent_fetches: self.config.max_concurrent_fetches,
            },
        }
    }
}

// Static Prometheus gauges
static DEX_PROGS_CACHED: Lazy<IntGauge> = Lazy::new(|| {
    prometheus::register_int_gauge!(
        "dex_registry_programs_cached",
        "Current number of DEX programs cached"
    )
    .unwrap()
});
static TOKENS_CACHED: Lazy<IntGauge> = Lazy::new(|| {
    prometheus::register_int_gauge!(
        "dex_registry_tokens_cached",
        "Current number of tokens cached"
    )
    .unwrap()
});
static PLUGINS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    prometheus::register_int_gauge!("dex_registry_plugins_total", "Total loaded plugins").unwrap()
});
static LAST_HEALTH_CHECK: Lazy<IntGauge> = Lazy::new(|| {
    prometheus::register_int_gauge!(
        "dex_registry_last_health_check_unix",
        "Last health check UNIX timestamp"
    )
    .unwrap()
});

#[derive(Debug, Clone)]
pub struct RegistryServiceStats {
    pub cache_stats: crate::cache::CacheStats,
    pub plugin_stats: crate::plugin::PluginStats,
    pub config_summary: ConfigSummary,
}

#[derive(Debug, Clone)]
pub struct ConfigSummary {
    pub cache_ttl_seconds: u64,
    pub auto_load_plugins: bool,
    pub discovery_batch_size: usize,
    pub max_concurrent_fetches: usize,
}
