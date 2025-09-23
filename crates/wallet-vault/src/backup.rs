use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use tar::{Archive, Builder};
use uuid::Uuid;

use crate::error::{Result, VaultError};
use crate::wallet::WalletExport;

#[derive(Clone)]
pub struct BackupManager {
    backup_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct BackupManifest {
    backup_id: String,
    created_at: chrono::DateTime<Utc>,
    wallet_count: usize,
    version: u32,
}

impl BackupManager {
    pub fn new(backup_dir: PathBuf) -> Self {
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).ok();
        }
        BackupManager { backup_dir }
    }

    pub async fn create_backup(&self, wallets: Vec<WalletExport>) -> Result<String> {
        let backup_id = Uuid::new_v4().to_string();
        let backup_path = self.backup_dir.join(&backup_id);

        fs::create_dir_all(&backup_path).map_err(|e| VaultError::BackupFailed(e.to_string()))?;

        let manifest = BackupManifest {
            backup_id: backup_id.clone(),
            created_at: Utc::now(),
            wallet_count: wallets.len(),
            version: 1,
        };

        let manifest_path = backup_path.join("manifest.json");
        let manifest_data = serde_json::to_string_pretty(&manifest)?;
        fs::write(manifest_path, manifest_data)
            .map_err(|e| VaultError::BackupFailed(e.to_string()))?;

        for (i, wallet) in wallets.iter().enumerate() {
            let wallet_file = backup_path.join(format!("wallet_{}.json", i));
            let wallet_data = serde_json::to_string_pretty(wallet)?;
            fs::write(wallet_file, wallet_data)
                .map_err(|e| VaultError::BackupFailed(e.to_string()))?;
        }

        let archive_path = self.backup_dir.join(format!("{}.tar.gz", backup_id));
        let tar_gz =
            fs::File::create(&archive_path).map_err(|e| VaultError::BackupFailed(e.to_string()))?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(enc);

        tar.append_dir_all(&backup_id, &backup_path)
            .map_err(|e| VaultError::BackupFailed(e.to_string()))?;

        tar.finish()
            .map_err(|e| VaultError::BackupFailed(e.to_string()))?;

        fs::remove_dir_all(backup_path).map_err(|e| VaultError::BackupFailed(e.to_string()))?;

        Ok(backup_id)
    }

    pub async fn restore_backup(&self, backup_id: &str) -> Result<Vec<WalletExport>> {
        let archive_path = self.backup_dir.join(format!("{}.tar.gz", backup_id));

        if !archive_path.exists() {
            return Err(VaultError::RestoreFailed(format!(
                "Backup {} not found",
                backup_id
            )));
        }

        let tar_gz =
            fs::File::open(&archive_path).map_err(|e| VaultError::RestoreFailed(e.to_string()))?;
        let dec = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(dec);

        let temp_dir = self.backup_dir.join(format!("restore_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).map_err(|e| VaultError::RestoreFailed(e.to_string()))?;

        archive
            .unpack(&temp_dir)
            .map_err(|e| VaultError::RestoreFailed(e.to_string()))?;

        let backup_path = temp_dir.join(backup_id);
        let manifest_path = backup_path.join("manifest.json");

        let manifest_data = fs::read_to_string(manifest_path)
            .map_err(|e| VaultError::RestoreFailed(e.to_string()))?;
        let manifest: BackupManifest = serde_json::from_str(&manifest_data)?;

        let mut wallets = Vec::new();

        for i in 0..manifest.wallet_count {
            let wallet_file = backup_path.join(format!("wallet_{}.json", i));
            let wallet_data = fs::read_to_string(wallet_file)
                .map_err(|e| VaultError::RestoreFailed(e.to_string()))?;
            let wallet: WalletExport = serde_json::from_str(&wallet_data)?;
            wallets.push(wallet);
        }

        fs::remove_dir_all(temp_dir).map_err(|e| VaultError::RestoreFailed(e.to_string()))?;

        Ok(wallets)
    }

    pub async fn list_backups(&self) -> Result<Vec<(String, chrono::DateTime<Utc>)>> {
        let mut backups = Vec::new();

        let entries =
            fs::read_dir(&self.backup_dir).map_err(|e| VaultError::BackupFailed(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| VaultError::BackupFailed(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("gz") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(backup_id) = filename.strip_suffix(".tar") {
                        let metadata = entry
                            .metadata()
                            .map_err(|e| VaultError::BackupFailed(e.to_string()))?;

                        if let Ok(modified) = metadata.modified() {
                            let datetime = chrono::DateTime::<Utc>::from(modified);
                            backups.push((backup_id.to_string(), datetime));
                        }
                    }
                }
            }
        }

        backups.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(backups)
    }

    pub async fn delete_backup(&self, backup_id: &str) -> Result<()> {
        let archive_path = self.backup_dir.join(format!("{}.tar.gz", backup_id));

        if archive_path.exists() {
            fs::remove_file(archive_path).map_err(|e| VaultError::BackupFailed(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn cleanup_old_backups(&self, keep_count: usize) -> Result<usize> {
        let backups = self.list_backups().await?;
        let mut deleted = 0;

        for (backup_id, _) in backups.iter().skip(keep_count) {
            self.delete_backup(backup_id).await?;
            deleted += 1;
        }

        Ok(deleted)
    }
}
