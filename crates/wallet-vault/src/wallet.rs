use base64::engine::general_purpose;
use base64::Engine;
use chrono::{DateTime, Utc};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::encryption::DualEncryption;
use crate::error::{Result, VaultError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub enum WalletTier {
    Hot,  // Active trading, low limits
    Warm, // Reserve funds, medium limits
    Cold, // Long-term storage, no limits
}

impl WalletTier {
    pub fn max_balance_lamports(&self) -> u64 {
        match self {
            WalletTier::Hot => 100_000_000,     // 0.1 SOL
            WalletTier::Warm => 10_000_000_000, // 10 SOL
            WalletTier::Cold => u64::MAX,       // No limit
        }
    }

    pub fn daily_transaction_limit(&self) -> u32 {
        match self {
            WalletTier::Hot => 100,
            WalletTier::Warm => 20,
            WalletTier::Cold => 5,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub id: Uuid,
    pub pubkey: String,
    pub tier: WalletTier,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub access_count: u64,
    pub is_active: bool,
    pub daily_transactions: u32,
    pub total_volume_lamports: u64,
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecureWallet {
    #[zeroize(skip)]
    pub metadata: WalletMetadata,
    encrypted_keypair: Vec<u8>,
}

impl SecureWallet {
    pub fn new(
        keypair: Keypair,
        tier: WalletTier,
        label: Option<String>,
        encryption: &DualEncryption,
    ) -> Result<Self> {
        let keypair_bytes = keypair.to_bytes();
        let encrypted_keypair = encryption.double_encrypt(&keypair_bytes)?;

        let metadata = WalletMetadata {
            id: Uuid::new_v4(),
            pubkey: keypair.pubkey().to_string(),
            tier,
            label,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            is_active: true,
            daily_transactions: 0,
            total_volume_lamports: 0,
        };

        Ok(SecureWallet {
            metadata,
            encrypted_keypair,
        })
    }

    pub fn decrypt_keypair(&mut self, encryption: &DualEncryption) -> Result<Keypair> {
        let decrypted = encryption.double_decrypt(&self.encrypted_keypair)?;
        let keypair = Keypair::from_bytes(decrypted.expose_secret())
            .map_err(|e| VaultError::Decryption(e.to_string()))?;

        self.metadata.last_accessed = Some(Utc::now());
        self.metadata.access_count += 1;

        Ok(keypair)
    }

    pub fn can_transact(&self, amount_lamports: u64) -> bool {
        let within_balance = amount_lamports <= self.metadata.tier.max_balance_lamports();
        let within_daily_limit =
            self.metadata.daily_transactions < self.metadata.tier.daily_transaction_limit();

        self.metadata.is_active && within_balance && within_daily_limit
    }

    pub fn record_transaction(&mut self, amount_lamports: u64) {
        self.metadata.daily_transactions += 1;
        self.metadata.total_volume_lamports += amount_lamports;
        self.metadata.updated_at = Utc::now();
    }

    pub fn reset_daily_limits(&mut self) {
        self.metadata.daily_transactions = 0;
    }

    pub fn rotate_keypair(
        &mut self,
        new_keypair: Keypair,
        encryption: &DualEncryption,
    ) -> Result<()> {
        let keypair_bytes = new_keypair.to_bytes();
        self.encrypted_keypair = encryption.double_encrypt(&keypair_bytes)?;
        self.metadata.pubkey = new_keypair.pubkey().to_string();
        self.metadata.updated_at = Utc::now();
        Ok(())
    }

    pub fn encrypted_keypair(&self) -> &[u8] {
        &self.encrypted_keypair
    }

    pub fn from_database(metadata: WalletMetadata, encrypted_keypair: Vec<u8>) -> Self {
        SecureWallet {
            metadata,
            encrypted_keypair,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WalletExport {
    pub metadata: WalletMetadata,
    pub encrypted_data: String,
    pub export_version: u32,
    pub exported_at: DateTime<Utc>,
}

impl SecureWallet {
    pub fn export(&self) -> WalletExport {
        WalletExport {
            metadata: self.metadata.clone(),
            encrypted_data: general_purpose::STANDARD.encode(&self.encrypted_keypair),
            export_version: 1,
            exported_at: Utc::now(),
        }
    }

    pub fn from_export(export: WalletExport) -> Result<Self> {
        let encrypted_keypair = general_purpose::STANDARD
            .decode(&export.encrypted_data)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        Ok(SecureWallet {
            metadata: export.metadata,
            encrypted_keypair,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::MasterKey;

    #[test]
    fn test_wallet_creation() {
        let master_key = MasterKey::generate();
        let encryption = DualEncryption::new(&master_key).unwrap();
        let keypair = Keypair::new();

        let wallet = SecureWallet::new(
            keypair,
            WalletTier::Hot,
            Some("Test Wallet".to_string()),
            &encryption,
        )
        .unwrap();

        assert_eq!(wallet.metadata.tier, WalletTier::Hot);
        assert_eq!(wallet.metadata.label, Some("Test Wallet".to_string()));
        assert!(wallet.metadata.is_active);
    }

    #[test]
    fn test_wallet_encryption_decryption() {
        let master_key = MasterKey::generate();
        let encryption = DualEncryption::new(&master_key).unwrap();
        let original_keypair = Keypair::new();
        let original_pubkey = original_keypair.pubkey();

        let mut wallet =
            SecureWallet::new(original_keypair, WalletTier::Warm, None, &encryption).unwrap();

        let decrypted_keypair = wallet.decrypt_keypair(&encryption).unwrap();
        assert_eq!(decrypted_keypair.pubkey(), original_pubkey);
        assert_eq!(wallet.metadata.access_count, 1);
        assert!(wallet.metadata.last_accessed.is_some());
    }

    #[test]
    fn test_transaction_limits() {
        let master_key = MasterKey::generate();
        let encryption = DualEncryption::new(&master_key).unwrap();
        let keypair = Keypair::new();

        let wallet = SecureWallet::new(keypair, WalletTier::Hot, None, &encryption).unwrap();

        assert!(wallet.can_transact(50_000_000)); // 0.05 SOL
        assert!(!wallet.can_transact(200_000_000)); // 0.2 SOL (exceeds limit)
    }
}
