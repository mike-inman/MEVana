// Library interface for hotpath_arb_engine
// Exposes modules needed by monitoring-service and other consumers

pub mod event_bus;
pub mod orderbook_tracker;
pub mod smart_filter;
pub mod property_store;
pub mod execution_integration;
pub mod live_trading_config;
pub mod live_wallet_manager;
pub mod jito_integration;
pub mod flash_loan_integration;
pub mod marginfi_instructions;
pub mod transaction_templates;
pub mod config_api;
pub mod ai_strategy_engine;
pub mod market_intelligence;
pub mod intelligent_execution_core;
