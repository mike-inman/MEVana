use std::env;
use std::path::PathBuf;
use tokio::net::TcpListener;
use wallet_vault::{
    api::create_api_router,
    vault::{VaultConfig, WalletVault},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let port = env::var("VAULT_PORT").unwrap_or_else(|_| "8090".to_string());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let master_password = env::var("VAULT_MASTER_PASSWORD")
        .unwrap_or_else(|_| "mevana_dev_password_change_in_production".to_string());

    log::info!("Starting wallet-vault service on port {}", port);
    log::info!(
        "Connecting to database: {}",
        database_url.replace(":mevpassword@", ":***@")
    );

    // Create vault configuration
    let config = VaultConfig {
        database_url,
        backup_path: PathBuf::from("./vault_backups"),
        auto_backup: true,
        backup_interval_hours: 24,
        max_hot_wallets: 10,
        max_warm_wallets: 5,
        max_cold_wallets: 3,
        enable_hsm: false,
        hsm_endpoint: None,
    };

    // Initialize the wallet vault
    let vault = WalletVault::new(config, &master_password).await?;

    // Create the API router
    let app = create_api_router(vault);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    log::info!("🔐 Wallet Vault Service running on http://0.0.0.0:{}", port);
    log::info!(
        "📊 Swagger UI available at http://localhost:{}/swagger-ui/",
        port
    );
    log::info!("📱 Endpoints:");
    log::info!("   - GET  /wallets          - List wallets");
    log::info!("   - POST /wallets          - Create wallet");
    log::info!("   - GET  /wallets/:id      - Get wallet details");
    log::info!("   - POST /backup           - Create backup");
    log::info!("   - GET  /backups          - List backups");

    axum::serve(listener, app).await?;

    Ok(())
}
