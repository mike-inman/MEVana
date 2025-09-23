#![allow(warnings)]

// Library interface for hotpath_arb_engine
// Exposes modules needed by monitoring-service and other consumers

pub mod config_api;
pub mod event_bus;
pub mod execution_integration;
pub mod flash_loan_integration;
pub mod jito_integration;
pub mod live_trading_config;
pub mod live_wallet_manager;
pub mod marginfi_instructions;
pub mod orderbook_tracker;
pub mod property_store;
pub mod smart_filter;
pub mod transaction_templates;

#[cfg(feature = "experimental-hooks")]
pub mod ai_strategy_engine;
#[cfg(feature = "experimental-hooks")]
pub mod intelligent_execution_core;
#[cfg(feature = "experimental-hooks")]
pub mod market_intelligence;
