pub mod vault;
pub mod encryption;
pub mod wallet;
pub mod api;
pub mod backup;
pub mod error;
pub mod auth;

pub use vault::WalletVault;
pub use wallet::{SecureWallet, WalletTier};
pub use api::create_api_router;
pub use error::{VaultError, Result};
