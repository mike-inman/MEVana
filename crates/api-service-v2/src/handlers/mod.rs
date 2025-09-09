//! API request handlers
//!
//! HTTP request handlers for all API endpoints

pub mod auth;
pub mod health;
pub mod users;
pub mod wallets;
pub mod trading;

// Re-export commonly used handlers
pub use health::health_check;