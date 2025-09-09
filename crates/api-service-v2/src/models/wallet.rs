//! Wallet-related models

use crate::database::Entity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Wallet entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub wallet_type: WalletType,
    pub public_key: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Entity for Wallet {
    type Id = Uuid;
    
    fn id(&self) -> &Self::Id {
        &self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = id;
    }
}

/// Wallet type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum WalletType {
    Hot,
    Warm,
    Cold,
}

/// Wallet creation request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct WalletCreate {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub wallet_type: WalletType,
}

/// Wallet update request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct WalletUpdate {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

/// Wallet response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletResponse {
    pub id: Uuid,
    pub name: String,
    pub wallet_type: WalletType,
    pub public_key: String,
    pub balance: Option<WalletBalance>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Wallet> for WalletResponse {
    fn from(wallet: Wallet) -> Self {
        Self {
            id: wallet.id,
            name: wallet.name,
            wallet_type: wallet.wallet_type,
            public_key: wallet.public_key,
            balance: None, // Would be populated by service layer
            is_active: wallet.is_active,
            created_at: wallet.created_at,
            updated_at: wallet.updated_at,
        }
    }
}

/// Wallet balance information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletBalance {
    pub sol_balance: f64,
    pub token_balances: Vec<TokenBalance>,
    pub total_value_usd: Option<f64>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Token balance information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TokenBalance {
    pub mint: String,
    pub symbol: String,
    pub amount: f64,
    pub decimals: u8,
    pub value_usd: Option<f64>,
}

/// Wallet transaction
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub signature: String,
    pub transaction_type: TransactionType,
    pub amount: f64,
    pub token_mint: Option<String>,
    pub fee: f64,
    pub status: TransactionStatus,
    pub block_time: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Transaction type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TransactionType {
    Send,
    Receive,
    Swap,
    Stake,
    Unstake,
    Trade,
}

/// Transaction status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}