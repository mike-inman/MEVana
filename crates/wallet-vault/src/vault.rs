use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use solana_sdk::signature::Keypair;
use sqlx::{PgPool, Row};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::backup::BackupManager;
use crate::encryption::{DualEncryption, MasterKey};
use crate::error::{Result, VaultError};
use crate::wallet::{SecureWallet, WalletMetadata, WalletTier};

#[derive(Clone)]
pub struct VaultConfig {
    pub database_url: String,
    pub backup_path: PathBuf,
    pub auto_backup: bool,
    pub backup_interval_hours: u64,
    pub max_hot_wallets: usize,
    pub max_warm_wallets: usize,
    pub max_cold_wallets: usize,
    pub enable_hsm: bool,
    pub hsm_endpoint: Option<String>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        VaultConfig {
            database_url: "postgresql://localhost/wallet_vault".to_string(),
            backup_path: PathBuf::from("/var/lib/mevana/vault/backups"),
            auto_backup: true,
            backup_interval_hours: 24,
            max_hot_wallets: 10,
            max_warm_wallets: 5,
            max_cold_wallets: 3,
            enable_hsm: false,
            hsm_endpoint: None,
        }
    }
}

pub struct WalletVault {
    config: VaultConfig,
    pool: PgPool,
    encryption: Arc<DualEncryption>,
    wallets: Arc<RwLock<HashMap<Uuid, SecureWallet>>>,
    pub backup_manager: BackupManager,
}

impl WalletVault {
    pub async fn new(config: VaultConfig, master_password: &str) -> Result<Self> {
        let pool = PgPool::connect(&config.database_url).await?;

        let salt = Self::get_or_create_salt(&pool).await?;
        let master_key = MasterKey::derive_from_password(master_password, &salt)?;
        let encryption = Arc::new(DualEncryption::new(&master_key)?);

        let backup_manager = BackupManager::new(config.backup_path.clone());

        let vault = WalletVault {
            config,
            pool,
            encryption,
            wallets: Arc::new(RwLock::new(HashMap::new())),
            backup_manager,
        };

        vault.load_wallets_from_db().await?;

        if vault.config.auto_backup {
            vault.start_auto_backup().await;
        }

        Ok(vault)
    }

    async fn get_or_create_salt(pool: &PgPool) -> Result<Vec<u8>> {
        let row = sqlx::query("SELECT salt FROM vault_config WHERE id = 1")
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            Ok(row.get("salt"))
        } else {
            let salt = SaltString::generate(&mut OsRng);
            let salt_bytes = salt.as_str().as_bytes().to_vec();

            sqlx::query("INSERT INTO vault_config (id, salt) VALUES (1, $1)")
                .bind(&salt_bytes)
                .execute(pool)
                .await?;

            Ok(salt_bytes)
        }
    }

    async fn load_wallets_from_db(&self) -> Result<()> {
        let rows = sqlx::query("SELECT * FROM wallets WHERE is_active = true")
            .fetch_all(&self.pool)
            .await?;

        let mut wallets = self.wallets.write().await;

        for row in rows {
            let id: Uuid = row.get("id");
            let encrypted_data: Vec<u8> = row.get("encrypted_keypair");
            let tier_str: String = row.get("tier");

            let tier = match tier_str.as_str() {
                "hot" => WalletTier::Hot,
                "warm" => WalletTier::Warm,
                "cold" => WalletTier::Cold,
                _ => continue,
            };

            let metadata = WalletMetadata {
                id,
                pubkey: row.get("pubkey"),
                tier,
                label: row.get("label"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_accessed: row.get("last_accessed"),
                access_count: row.get::<i64, _>("access_count") as u64,
                is_active: row.get("is_active"),
                daily_transactions: row.get::<i32, _>("daily_transactions") as u32,
                total_volume_lamports: row.get::<i64, _>("total_volume_lamports") as u64,
            };

            let wallet = SecureWallet::from_database(metadata, encrypted_data);

            wallets.insert(id, wallet);
        }

        Ok(())
    }

    pub async fn create_wallet(
        &self,
        tier: WalletTier,
        label: Option<String>,
    ) -> Result<WalletMetadata> {
        let wallet_count = self.count_wallets_by_tier(&tier).await?;
        let max_allowed = match tier {
            WalletTier::Hot => self.config.max_hot_wallets,
            WalletTier::Warm => self.config.max_warm_wallets,
            WalletTier::Cold => self.config.max_cold_wallets,
        };

        if wallet_count >= max_allowed {
            return Err(VaultError::ConfigError(format!(
                "Maximum {} wallets ({}) reached",
                format!("{:?}", tier).to_lowercase(),
                max_allowed
            )));
        }

        let keypair = Keypair::new();
        let wallet = SecureWallet::new(keypair, tier.clone(), label.clone(), &self.encryption)?;

        self.save_wallet_to_db(&wallet).await?;

        let metadata = wallet.metadata.clone();
        let mut wallets = self.wallets.write().await;
        wallets.insert(metadata.id, wallet);

        Ok(metadata)
    }

    async fn save_wallet_to_db(&self, wallet: &SecureWallet) -> Result<()> {
        let tier_str = match wallet.metadata.tier {
            WalletTier::Hot => "hot",
            WalletTier::Warm => "warm",
            WalletTier::Cold => "cold",
        };

        sqlx::query(
            "INSERT INTO wallets (
                id, pubkey, tier, label, encrypted_keypair,
                created_at, updated_at, last_accessed, access_count,
                is_active, daily_transactions, total_volume_lamports
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                pubkey = EXCLUDED.pubkey,
                encrypted_keypair = EXCLUDED.encrypted_keypair,
                updated_at = EXCLUDED.updated_at,
                last_accessed = EXCLUDED.last_accessed,
                access_count = EXCLUDED.access_count,
                daily_transactions = EXCLUDED.daily_transactions,
                total_volume_lamports = EXCLUDED.total_volume_lamports",
        )
        .bind(wallet.metadata.id)
        .bind(&wallet.metadata.pubkey)
        .bind(tier_str)
        .bind(&wallet.metadata.label)
        .bind(wallet.encrypted_keypair())
        .bind(wallet.metadata.created_at)
        .bind(wallet.metadata.updated_at)
        .bind(wallet.metadata.last_accessed)
        .bind(wallet.metadata.access_count as i64)
        .bind(wallet.metadata.is_active)
        .bind(wallet.metadata.daily_transactions as i32)
        .bind(wallet.metadata.total_volume_lamports as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_wallet_metadata(&self, id: Uuid) -> Result<WalletMetadata> {
        let wallets = self.wallets.read().await;
        wallets
            .get(&id)
            .map(|w| w.metadata.clone())
            .ok_or_else(|| VaultError::WalletNotFound(id.to_string()))
    }

    pub async fn get_keypair(&self, id: Uuid) -> Result<Keypair> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets
            .get_mut(&id)
            .ok_or_else(|| VaultError::WalletNotFound(id.to_string()))?;

        let keypair = wallet.decrypt_keypair(&self.encryption)?;
        self.save_wallet_to_db(wallet).await?;
        Ok(keypair)
    }

    pub async fn list_wallets(&self) -> Result<Vec<WalletMetadata>> {
        let wallets = self.wallets.read().await;
        Ok(wallets.values().map(|w| w.metadata.clone()).collect())
    }

    pub async fn list_wallets_by_tier(&self, tier: WalletTier) -> Result<Vec<WalletMetadata>> {
        let wallets = self.wallets.read().await;
        Ok(wallets
            .values()
            .filter(|w| w.metadata.tier == tier)
            .map(|w| w.metadata.clone())
            .collect())
    }

    async fn count_wallets_by_tier(&self, tier: &WalletTier) -> Result<usize> {
        let wallets = self.wallets.read().await;
        Ok(wallets
            .values()
            .filter(|w| w.metadata.tier == *tier)
            .count())
    }

    pub async fn rotate_wallet(&self, id: Uuid) -> Result<WalletMetadata> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets
            .get_mut(&id)
            .ok_or_else(|| VaultError::WalletNotFound(id.to_string()))?;

        let new_keypair = Keypair::new();
        wallet.rotate_keypair(new_keypair, &self.encryption)?;
        self.save_wallet_to_db(wallet).await?;

        Ok(wallet.metadata.clone())
    }

    pub async fn deactivate_wallet(&self, id: Uuid) -> Result<()> {
        let mut wallets = self.wallets.write().await;

        if let Some(wallet) = wallets.get_mut(&id) {
            wallet.metadata.is_active = false;
            wallet.metadata.updated_at = Utc::now();
            self.save_wallet_to_db(wallet).await?;
        }

        wallets.remove(&id);
        Ok(())
    }

    pub async fn backup(&self) -> Result<String> {
        let wallets = self.wallets.read().await;
        let wallet_exports: Vec<_> = wallets.values().map(|w| w.export()).collect();

        let backup_id = self.backup_manager.create_backup(wallet_exports).await?;

        sqlx::query(
            "INSERT INTO backup_history (backup_id, wallet_count, created_at)
             VALUES ($1, $2, $3)",
        )
        .bind(&backup_id)
        .bind(wallets.len() as i32)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(backup_id)
    }

    pub async fn restore(&self, backup_id: &str) -> Result<usize> {
        let wallet_exports = self.backup_manager.restore_backup(backup_id).await?;

        let mut wallets = self.wallets.write().await;
        wallets.clear();

        for export in wallet_exports {
            let wallet = SecureWallet::from_export(export)?;
            self.save_wallet_to_db(&wallet).await?;
            wallets.insert(wallet.metadata.id, wallet);
        }

        Ok(wallets.len())
    }

    async fn start_auto_backup(&self) {
        let vault = self.clone();
        let interval = std::time::Duration::from_secs(self.config.backup_interval_hours * 3600);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                if let Err(e) = vault.backup().await {
                    log::error!("Auto-backup failed: {}", e);
                }
            }
        });
    }

    pub async fn reset_daily_limits(&self) -> Result<()> {
        let mut wallets = self.wallets.write().await;

        for wallet in wallets.values_mut() {
            wallet.reset_daily_limits();
            self.save_wallet_to_db(wallet).await?;
        }

        Ok(())
    }
}

use argon2::password_hash::{rand_core::OsRng, SaltString};

impl Clone for WalletVault {
    fn clone(&self) -> Self {
        WalletVault {
            config: self.config.clone(),
            pool: self.pool.clone(),
            encryption: self.encryption.clone(),
            wallets: self.wallets.clone(),
            backup_manager: self.backup_manager.clone(),
        }
    }
}
