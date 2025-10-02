pub mod api;
pub mod auth;
pub mod backup;
pub mod encryption;
pub mod error;
pub mod vault;
pub mod wallet;

pub use api::create_api_router;
pub use error::{Result, VaultError};
pub use vault::WalletVault;
pub use wallet::{SecureWallet, WalletTier};
