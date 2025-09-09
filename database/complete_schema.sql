-- MEVana Complete Database Schema
-- This script creates all necessary tables for the MEVana system including:
-- - Wallet management and security
-- - Orderbook tracking and history  
-- - Arbitrage opportunities and execution results

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- ==============================================================================
-- CORE SYSTEM TABLES
-- ==============================================================================

-- 1. Vault configuration for wallet encryption
CREATE TABLE vault_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    salt BYTEA NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2. Secure wallet storage
CREATE TABLE wallets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pubkey VARCHAR(50) UNIQUE NOT NULL,
    tier VARCHAR(10) NOT NULL CHECK (tier IN ('hot', 'warm', 'cold')),
    label VARCHAR(255),
    encrypted_keypair BYTEA NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_accessed TIMESTAMPTZ,
    access_count BIGINT DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    daily_transactions INTEGER DEFAULT 0,
    total_volume_lamports BIGINT DEFAULT 0
);

-- 3. Backup history tracking
CREATE TABLE backup_history (
    id SERIAL PRIMARY KEY,
    backup_id VARCHAR(100) UNIQUE NOT NULL,
    wallet_count INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ==============================================================================
-- TRADING AND ARBITRAGE TABLES
-- ==============================================================================

-- 4. Orderbook snapshots for historical analysis
CREATE TABLE orderbook_snapshots (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    slot BIGINT NOT NULL,
    venue VARCHAR(50) NOT NULL,
    market VARCHAR(100) NOT NULL,
    best_bid DECIMAL(20,8),
    best_ask DECIMAL(20,8),
    mid_price DECIMAL(20,8),
    spread_bps INTEGER,
    bid_depth_usd DECIMAL(15,2),
    ask_depth_usd DECIMAL(15,2),
    total_bids INTEGER NOT NULL DEFAULT 0,
    total_asks INTEGER NOT NULL DEFAULT 0,
    raw_orderbook JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 5. Arbitrage opportunities detected
CREATE TABLE arbitrage_opportunities (
    id BIGSERIAL PRIMARY KEY,
    opportunity_id VARCHAR(100) UNIQUE NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    slot BIGINT NOT NULL,
    venue_sell VARCHAR(50) NOT NULL,
    venue_buy VARCHAR(50) NOT NULL,
    token_pair VARCHAR(100) NOT NULL,
    profit_bps INTEGER NOT NULL,
    amount_in DOUBLE PRECISION NOT NULL,
    expected_amount_out DOUBLE PRECISION NOT NULL,
    price_impact_pct DOUBLE PRECISION,
    detection_time_us BIGINT NOT NULL,
    strategy_type VARCHAR(50) NOT NULL,
    raw_opportunity JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 6. Execution results and performance
CREATE TABLE execution_results (
    id BIGSERIAL PRIMARY KEY,
    opportunity_id VARCHAR(100) NOT NULL REFERENCES arbitrage_opportunities(opportunity_id),
    timestamp TIMESTAMPTZ NOT NULL,
    executed BOOLEAN NOT NULL,
    success BOOLEAN,
    actual_profit_bps INTEGER,
    execution_time_us BIGINT,
    gas_cost_sol DOUBLE PRECISION,
    slippage_bps INTEGER,
    failure_reason TEXT,
    transaction_signature VARCHAR(100),
    bundle_id VARCHAR(100),
    raw_result JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 7. Venue performance aggregated statistics
CREATE TABLE venue_performance (
    id BIGSERIAL PRIMARY KEY,
    venue_pair VARCHAR(100) NOT NULL, -- 'meteora->orca'
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    total_opportunities INTEGER DEFAULT 0,
    executed_opportunities INTEGER DEFAULT 0,
    successful_executions INTEGER DEFAULT 0,
    total_profit_bps BIGINT DEFAULT 0,
    average_profit_bps DECIMAL(8,2),
    average_execution_time_us DECIMAL(10,2),
    success_rate_pct DECIMAL(5,2),
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(venue_pair, date)
);

-- ==============================================================================
-- API INTEGRATION TABLES
-- ==============================================================================

-- 8. User management for API access
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    clerk_user_id VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    role VARCHAR(50) DEFAULT 'trader' CHECK (role IN ('admin', 'trader', 'viewer')),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login TIMESTAMPTZ
);

-- 9. API sessions and JWT tracking
CREATE TABLE api_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    session_token VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_accessed TIMESTAMPTZ DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT
);

-- 10. Trading performance by user
CREATE TABLE user_trading_stats (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    opportunities_viewed INTEGER DEFAULT 0,
    manual_trades INTEGER DEFAULT 0,
    api_calls INTEGER DEFAULT 0,
    total_profit_bps BIGINT DEFAULT 0,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id, date)
);

-- ==============================================================================
-- INDEXES FOR PERFORMANCE
-- ==============================================================================

-- Wallet indexes
CREATE INDEX idx_wallets_tier ON wallets(tier);
CREATE INDEX idx_wallets_pubkey ON wallets(pubkey);
CREATE INDEX idx_wallets_is_active ON wallets(is_active);

-- Orderbook indexes
CREATE INDEX idx_orderbook_timestamp ON orderbook_snapshots(timestamp);
CREATE INDEX idx_orderbook_venue_market ON orderbook_snapshots(venue, market);
CREATE INDEX idx_orderbook_slot ON orderbook_snapshots(slot);

-- Arbitrage indexes
CREATE INDEX idx_arbitrage_timestamp ON arbitrage_opportunities(timestamp);
CREATE INDEX idx_arbitrage_venues ON arbitrage_opportunities(venue_sell, venue_buy);
CREATE INDEX idx_arbitrage_profit ON arbitrage_opportunities(profit_bps);
CREATE INDEX idx_arbitrage_token_pair ON arbitrage_opportunities(token_pair);

-- Execution indexes
CREATE INDEX idx_execution_timestamp ON execution_results(timestamp);
CREATE INDEX idx_execution_success ON execution_results(success);
CREATE INDEX idx_execution_opportunity ON execution_results(opportunity_id);

-- Venue performance indexes
CREATE INDEX idx_venue_performance_date ON venue_performance(date);
CREATE INDEX idx_venue_performance_pair ON venue_performance(venue_pair);

-- User and session indexes
CREATE INDEX idx_users_clerk_id ON users(clerk_user_id);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_sessions_user_id ON api_sessions(user_id);
CREATE INDEX idx_sessions_token ON api_sessions(session_token);
CREATE INDEX idx_sessions_expires ON api_sessions(expires_at);

-- Trading stats indexes
CREATE INDEX idx_user_stats_user_date ON user_trading_stats(user_id, date);

-- ==============================================================================
-- TOKEN AND DEX PROGRAM MANAGEMENT
-- ==============================================================================

-- 11. Token metadata and versions
CREATE TABLE tokens (
    id BIGSERIAL PRIMARY KEY,
    mint VARCHAR(50) UNIQUE NOT NULL,
    symbol VARCHAR(20),
    name VARCHAR(100),
    decimals INTEGER NOT NULL,
    logo_uri TEXT,
    is_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 12. Token version tracking for metadata updates
CREATE TABLE token_versions (
    id BIGSERIAL PRIMARY KEY,
    token_id BIGINT NOT NULL REFERENCES tokens(id),
    version INTEGER NOT NULL,
    symbol VARCHAR(20),
    name VARCHAR(100),
    decimals INTEGER NOT NULL,
    logo_uri TEXT,
    is_verified BOOLEAN DEFAULT FALSE,
    metadata_json JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(token_id, version)
);

-- 13. DEX program registry for supported protocols
CREATE TABLE dex_program_registry (
    id SERIAL PRIMARY KEY,
    program_id VARCHAR(50) UNIQUE NOT NULL,
    protocol_name VARCHAR(50) NOT NULL,
    protocol_version VARCHAR(20),
    is_active BOOLEAN DEFAULT TRUE,
    supported_operations JSONB NOT NULL, -- ['swap', 'add_liquidity', 'remove_liquidity']
    fee_structure JSONB, -- Protocol-specific fee information
    idl_version VARCHAR(20),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 14. User-Wallet associations (for multi-trader support)
CREATE TABLE user_wallets (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    wallet_id UUID NOT NULL REFERENCES wallets(id),
    role VARCHAR(20) DEFAULT 'trader' CHECK (role IN ('owner', 'trader', 'viewer')),
    is_primary BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id, wallet_id)
);

-- ==============================================================================
-- HOTPATH ENGINE TABLES (from migration)
-- ==============================================================================

-- 15. Flash loan usage tracking
CREATE TABLE IF NOT EXISTS flash_loan_history (
    id BIGSERIAL PRIMARY KEY,
    opportunity_id VARCHAR(100),
    provider VARCHAR(50) NOT NULL,
    amount BIGINT NOT NULL,
    fee_lamports BIGINT NOT NULL,
    success BOOLEAN NOT NULL,
    repayment_amount BIGINT,
    timestamp TIMESTAMPTZ NOT NULL,
    transaction_signature VARCHAR(100),
    user_id UUID REFERENCES users(id),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 16. Performance metrics aggregation
CREATE TABLE IF NOT EXISTS performance_metrics (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    processing_time_p50_us BIGINT,
    processing_time_p95_us BIGINT,
    processing_time_p99_us BIGINT,
    opportunities_per_minute INTEGER,
    execution_success_rate DOUBLE PRECISION,
    total_profit_captured_bps BIGINT,
    memory_usage_mb DOUBLE PRECISION,
    cpu_utilization DOUBLE PRECISION,
    metrics_json JSONB,
    user_id UUID REFERENCES users(id),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 17. AI strategy optimization history
CREATE TABLE IF NOT EXISTS ai_optimization_history (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    strategy_type VARCHAR(50) NOT NULL,
    parameters JSONB NOT NULL,
    performance_score DOUBLE PRECISION,
    profit_improvement_pct DOUBLE PRECISION,
    applied BOOLEAN DEFAULT false,
    user_id UUID REFERENCES users(id),
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ==============================================================================
-- ADDITIONAL INDEXES
-- ==============================================================================

-- Token indexes
CREATE INDEX idx_tokens_mint ON tokens(mint);
CREATE INDEX idx_tokens_symbol ON tokens(symbol);
CREATE INDEX idx_tokens_verified ON tokens(is_verified);

-- Token version indexes
CREATE INDEX idx_token_versions_token_id ON token_versions(token_id);
CREATE INDEX idx_token_versions_version ON token_versions(token_id, version);

-- DEX program registry indexes
CREATE INDEX idx_dex_programs_id ON dex_program_registry(program_id);
CREATE INDEX idx_dex_programs_active ON dex_program_registry(is_active);
CREATE INDEX idx_dex_programs_protocol ON dex_program_registry(protocol_name);

-- User-wallet association indexes
CREATE INDEX idx_user_wallets_user ON user_wallets(user_id);
CREATE INDEX idx_user_wallets_wallet ON user_wallets(wallet_id);
CREATE INDEX idx_user_wallets_primary ON user_wallets(user_id, is_primary);

-- HotPath engine indexes
CREATE INDEX IF NOT EXISTS idx_flash_timestamp ON flash_loan_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_flash_provider ON flash_loan_history(provider);
CREATE INDEX IF NOT EXISTS idx_flash_user ON flash_loan_history(user_id);

CREATE INDEX IF NOT EXISTS idx_perf_timestamp ON performance_metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_perf_user ON performance_metrics(user_id);

CREATE INDEX IF NOT EXISTS idx_ai_timestamp ON ai_optimization_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_ai_strategy ON ai_optimization_history(strategy_type);
CREATE INDEX IF NOT EXISTS idx_ai_user ON ai_optimization_history(user_id);

-- ==============================================================================
-- INITIAL DATA
-- ==============================================================================

-- Insert default vault configuration (will be updated with actual salt)
INSERT INTO vault_config (id, salt) VALUES (1, decode('placeholder_salt_will_be_replaced', 'hex'))
ON CONFLICT (id) DO NOTHING;

-- Create admin user (placeholder - will be replaced with actual Clerk data)
INSERT INTO users (clerk_user_id, email, name, role) VALUES 
('placeholder_admin', 'admin@mevana.com', 'System Administrator', 'admin')
ON CONFLICT (clerk_user_id) DO NOTHING;

-- Insert common DEX programs
INSERT INTO dex_program_registry (program_id, protocol_name, protocol_version, supported_operations, fee_structure) VALUES
('whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc', 'Orca Whirlpool', 'v1', '["swap", "add_liquidity", "remove_liquidity"]', '{"swap_fee_rate": "variable", "protocol_fee_rate": "0.0001"}'),
('675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8', 'Raydium AMM', 'v4', '["swap", "add_liquidity", "remove_liquidity"]', '{"swap_fee_rate": "0.0025", "protocol_fee_rate": "0.0001"}'),
('MERLuDFBMmsHnsBPZw2sDQZHvXFMwp8EdjudcU2HKky', 'Meteora', 'v1', '["swap", "add_liquidity", "remove_liquidity"]', '{"swap_fee_rate": "variable", "protocol_fee_rate": "0.0001"}')
ON CONFLICT (program_id) DO NOTHING;