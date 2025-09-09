use std::fmt;

#[derive(Debug)]
pub enum VaultError {
    Encryption(String),
    Decryption(String),
    Database(String),
    Serialization(String),
    InvalidPassword,
    WalletNotFound(String),
    AccessDenied,
    BackupFailed(String),
    RestoreFailed(String),
    HsmError(String),
    NetworkError(String),
    ConfigError(String),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            VaultError::Decryption(msg) => write!(f, "Decryption error: {}", msg),
            VaultError::Database(msg) => write!(f, "Database error: {}", msg),
            VaultError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            VaultError::InvalidPassword => write!(f, "Invalid password"),
            VaultError::WalletNotFound(id) => write!(f, "Wallet not found: {}", id),
            VaultError::AccessDenied => write!(f, "Access denied"),
            VaultError::BackupFailed(msg) => write!(f, "Backup failed: {}", msg),
            VaultError::RestoreFailed(msg) => write!(f, "Restore failed: {}", msg),
            VaultError::HsmError(msg) => write!(f, "HSM error: {}", msg),
            VaultError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            VaultError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<sqlx::Error> for VaultError {
    fn from(err: sqlx::Error) -> Self {
        VaultError::Database(err.to_string())
    }
}

impl From<serde_json::Error> for VaultError {
    fn from(err: serde_json::Error) -> Self {
        VaultError::Serialization(err.to_string())
    }
}

impl From<reqwest::Error> for VaultError {
    fn from(err: reqwest::Error) -> Self {
        VaultError::NetworkError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, VaultError>;