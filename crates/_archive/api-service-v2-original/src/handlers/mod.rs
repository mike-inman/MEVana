//! API request handlers
//!
//! HTTP request handlers for all API endpoints

pub mod arbitrage_simple;
pub mod auth;
pub mod health;

// Re-export commonly used handlers
pub use health::health_check;
pub use arbitrage_simple::{create_arbitrage_opportunity, get_arbitrage_opportunities};