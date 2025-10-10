-- MEVana Database Schema Creation Script
-- This script creates all necessary tables for the MEVana arbitrage system
-- Run this script on a clean PostgreSQL database

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
    timestamp TIMESTAMPTZ NOT NULL,
    slot BIGINT NOT NULL,
    venue VARCHAR(50) NOT NULL,
    market VARCHAR(100) NOT NULL,
    best_bid DOUBLE PRECISION,
    best_ask DOUBLE PRECISION,
    mid_price DOUBLE PRECISION,
    spread_bps INTEGER,
    bid_depth_usd DOUBLE PRECISION,
    ask_depth_usd DOUBLE PRECISION,
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
    wallet_id UUID REFERENCES wallets(id),
    raw_result JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 7. Flash loan usage tracking
CREATE TABLE flash_loan_history (
    id BIGSERIAL PRIMARY KEY,
    opportunity_id VARCHAR(100) REFERENCES arbitrage_opportunities(opportunity_id),
    provider VARCHAR(50) NOT NULL,
    amount BIGINT NOT NULL,
    fee_lamports BIGINT NOT NULL,
    success BOOLEAN NOT NULL,
    repayment_amount BIGINT,
    timestamp TIMESTAMPTZ NOT NULL,
    transaction_signature VARCHAR(100),
    wallet_id UUID REFERENCES wallets(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 8. Performance metrics aggregation
CREATE TABLE performance_metrics (
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
    active_wallets INTEGER,
    metrics_json JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 9. AI strategy optimization history
CREATE TABLE ai_optimization_history (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    strategy_type VARCHAR(50) NOT NULL,
    parameters JSONB NOT NULL,
    performance_score DOUBLE PRECISION,
    profit_improvement_pct DOUBLE PRECISION,
    applied BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ==============================================================================
-- TOKEN AND MARKET METADATA TABLES
-- ==============================================================================

-- 10. Token metadata registry
CREATE TABLE token_registry (
    id SERIAL PRIMARY KEY,
    mint VARCHAR(50) UNIQUE NOT NULL,
    symbol VARCHAR(20),
    name VARCHAR(100),
    decimals INTEGER NOT NULL,
    logo_uri VARCHAR(500),
    coingecko_id VARCHAR(100),
    is_verified BOOLEAN DEFAULT FALSE,
    market_cap_usd BIGINT,
    volume_24h_usd BIGINT,
    price_usd DOUBLE PRECISION,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 11. DEX venue configuration
CREATE TABLE dex_venues (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    program_id VARCHAR(50) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    fee_bps INTEGER NOT NULL DEFAULT 0,
    supported_features JSONB,
    api_endpoints JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 12. Trading pairs tracking
CREATE TABLE trading_pairs (
    id SERIAL PRIMARY KEY,
    venue_id INTEGER REFERENCES dex_venues(id),
    base_mint VARCHAR(50) NOT NULL,
    quote_mint VARCHAR(50) NOT NULL,
    pair_address VARCHAR(50),
    is_active BOOLEAN DEFAULT TRUE,
    min_order_size BIGINT,
    max_order_size BIGINT,
    tick_size DOUBLE PRECISION,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(venue_id, base_mint, quote_mint)
);

-- ==============================================================================
-- RESILIENCE FRAMEWORK TABLES
-- ==============================================================================

-- 13. Dead letter queue for failed events
CREATE TABLE dead_letter_queue (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payload JSONB NOT NULL,
    error TEXT NOT NULL,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_attempted_at TIMESTAMPTZ DEFAULT NOW(),
    source VARCHAR(100) NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- 14. Circuit breaker state tracking
CREATE TABLE circuit_breaker_state (
    id SERIAL PRIMARY KEY,
    service_name VARCHAR(100) UNIQUE NOT NULL,
    state VARCHAR(20) NOT NULL CHECK (state IN ('closed', 'open', 'half_open')),
    failure_count INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    last_failure TIMESTAMPTZ,
    last_success TIMESTAMPTZ,
    opened_at TIMESTAMPTZ,
    config JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 15. Service degradation tracking
CREATE TABLE service_levels (
    id SERIAL PRIMARY KEY,
    service_name VARCHAR(100) UNIQUE NOT NULL,
    priority INTEGER NOT NULL,
    enabled BOOLEAN DEFAULT TRUE,
    load_threshold DOUBLE PRECISION NOT NULL,
    fallback_strategy VARCHAR(50) NOT NULL,
    fallback_config JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 16. System load metrics
CREATE TABLE system_load_history (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cpu_usage DOUBLE PRECISION NOT NULL,
    memory_usage DOUBLE PRECISION NOT NULL,
    active_connections INTEGER NOT NULL,
    error_rate DOUBLE PRECISION NOT NULL,
    response_time_p99_ms INTEGER NOT NULL
);

-- ==============================================================================
-- AUDIT AND LOGGING TABLES
-- ==============================================================================

-- 17. Audit log for security events
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    user_id VARCHAR(100),
    wallet_id UUID REFERENCES wallets(id),
    action VARCHAR(100) NOT NULL,
    resource VARCHAR(100),
    resource_id VARCHAR(100),
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,
    success BOOLEAN NOT NULL,
    error_message TEXT
);

-- 18. Transaction monitoring
CREATE TABLE transaction_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    transaction_signature VARCHAR(100) UNIQUE NOT NULL,
    wallet_id UUID REFERENCES wallets(id),
    transaction_type VARCHAR(50) NOT NULL,
    amount_lamports BIGINT,
    fee_lamports BIGINT,
    status VARCHAR(20) NOT NULL,
    confirmation_slot BIGINT,
    block_height BIGINT,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ==============================================================================
-- INDEXES FOR PERFORMANCE
-- ==============================================================================

-- Orderbook snapshots indexes
CREATE INDEX idx_orderbook_timestamp ON orderbook_snapshots(timestamp DESC);
CREATE INDEX idx_orderbook_slot ON orderbook_snapshots(slot DESC);
CREATE INDEX idx_orderbook_venue_market ON orderbook_snapshots(venue, market);
CREATE INDEX idx_orderbook_venue_timestamp ON orderbook_snapshots(venue, timestamp DESC);

-- Arbitrage opportunities indexes
CREATE INDEX idx_arb_timestamp ON arbitrage_opportunities(timestamp DESC);
CREATE INDEX idx_arb_slot ON arbitrage_opportunities(slot DESC);
CREATE INDEX idx_arb_venues ON arbitrage_opportunities(venue_sell, venue_buy);
CREATE INDEX idx_arb_profit ON arbitrage_opportunities(profit_bps DESC);
CREATE INDEX idx_arb_token_pair ON arbitrage_opportunities(token_pair);

-- Execution results indexes
CREATE INDEX idx_exec_opportunity ON execution_results(opportunity_id);
CREATE INDEX idx_exec_timestamp ON execution_results(timestamp DESC);
CREATE INDEX idx_exec_success ON execution_results(success);
CREATE INDEX idx_exec_signature ON execution_results(transaction_signature);
CREATE INDEX idx_exec_wallet ON execution_results(wallet_id);

-- Performance metrics indexes
CREATE INDEX idx_perf_timestamp ON performance_metrics(timestamp DESC);

-- Wallet indexes
CREATE INDEX idx_wallet_tier ON wallets(tier);
CREATE INDEX idx_wallet_active ON wallets(is_active);
CREATE INDEX idx_wallet_pubkey ON wallets(pubkey);

-- Token registry indexes
CREATE INDEX idx_token_mint ON token_registry(mint);
CREATE INDEX idx_token_symbol ON token_registry(symbol);
CREATE INDEX idx_token_verified ON token_registry(is_verified);

-- Flash loan indexes
CREATE INDEX idx_flash_timestamp ON flash_loan_history(timestamp DESC);
CREATE INDEX idx_flash_provider ON flash_loan_history(provider);
CREATE INDEX idx_flash_wallet ON flash_loan_history(wallet_id);

-- AI optimization indexes
CREATE INDEX idx_ai_timestamp ON ai_optimization_history(timestamp DESC);
CREATE INDEX idx_ai_strategy ON ai_optimization_history(strategy_type);

-- Dead letter queue indexes
CREATE INDEX idx_dlq_created_at ON dead_letter_queue(created_at DESC);
CREATE INDEX idx_dlq_source ON dead_letter_queue(source);
CREATE INDEX idx_dlq_retry_count ON dead_letter_queue(retry_count);

-- Audit log indexes
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_event_type ON audit_log(event_type);
CREATE INDEX idx_audit_wallet_id ON audit_log(wallet_id);
CREATE INDEX idx_audit_user_id ON audit_log(user_id);

-- Transaction log indexes
CREATE INDEX idx_txn_timestamp ON transaction_log(timestamp DESC);
CREATE INDEX idx_txn_signature ON transaction_log(transaction_signature);
CREATE INDEX idx_txn_wallet_id ON transaction_log(wallet_id);
CREATE INDEX idx_txn_type ON transaction_log(transaction_type);

-- System load indexes
CREATE INDEX idx_system_load_timestamp ON system_load_history(timestamp DESC);

-- ==============================================================================
-- TRIGGERS AND FUNCTIONS
-- ==============================================================================

-- Update timestamp trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply update triggers
CREATE TRIGGER update_wallets_updated_at BEFORE UPDATE ON wallets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_token_registry_updated_at BEFORE UPDATE ON token_registry FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_dex_venues_updated_at BEFORE UPDATE ON dex_venues FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_trading_pairs_updated_at BEFORE UPDATE ON trading_pairs FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_service_levels_updated_at BEFORE UPDATE ON service_levels FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_circuit_breaker_updated_at BEFORE UPDATE ON circuit_breaker_state FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Wallet transaction count trigger
CREATE OR REPLACE FUNCTION increment_wallet_transactions()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.success = TRUE AND NEW.wallet_id IS NOT NULL THEN
        UPDATE wallets 
        SET daily_transactions = daily_transactions + 1,
            total_volume_lamports = total_volume_lamports + COALESCE(NEW.actual_profit_bps, 0)
        WHERE id = NEW.wallet_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_wallet_transaction_count AFTER INSERT ON execution_results FOR EACH ROW EXECUTE FUNCTION increment_wallet_transactions();

-- ==============================================================================
-- PARTITIONING FOR LARGE TABLES
-- ==============================================================================

-- Partition orderbook_snapshots by month
CREATE TABLE orderbook_snapshots_template (LIKE orderbook_snapshots INCLUDING ALL);
-- Note: In production, create monthly partitions like:
-- CREATE TABLE orderbook_snapshots_2024_01 PARTITION OF orderbook_snapshots FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- ==============================================================================
-- INITIAL DATA SETUP
-- ==============================================================================

-- Insert common DEX venues
INSERT INTO dex_venues (name, program_id, fee_bps, supported_features) VALUES
('Orca Whirlpools', 'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc', 25, '{"concentrated_liquidity": true, "flash_loans": true}'),
('Raydium AMM', '675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8', 25, '{"amm": true, "farms": true}'),
('Meteora DLMM', 'LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo', 20, '{"dynamic_amm": true}'),
('Jupiter Aggregator', 'JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4', 0, '{"aggregation": true, "best_price": true}'),
('Pump.fun AMM', '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', 100, '{"bonding_curve": true}');

-- Insert common tokens
INSERT INTO token_registry (mint, symbol, name, decimals, is_verified) VALUES
('So11111111111111111111111111111111111111112', 'SOL', 'Solana', 9, true),
('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'USDC', 'USD Coin', 6, true),
('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 'USDT', 'Tether USD', 6, true),
('mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So', 'mSOL', 'Marinade SOL', 9, true),
('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs', 'ETH', 'Ethereum', 8, true);

-- Insert default service levels for graceful degradation
INSERT INTO service_levels (service_name, priority, load_threshold, fallback_strategy, fallback_config) VALUES
('arbitrage_detection', 10, 0.8, 'cache', '{"ttl_seconds": 300}'),
('price_feed', 9, 0.7, 'cache', '{"ttl_seconds": 60}'),
('wallet_operations', 8, 0.9, 'disabled', '{}'),
('orderbook_sync', 7, 0.6, 'cache', '{"ttl_seconds": 120}'),
('trade_execution', 10, 0.95, 'disabled', '{}'),
('metrics_collection', 3, 0.5, 'cache', '{"ttl_seconds": 600}');

-- ==============================================================================
-- VIEWS FOR COMMON QUERIES
-- ==============================================================================

-- Trading performance summary view
CREATE VIEW trading_performance_summary AS
SELECT 
    DATE(er.created_at) as trading_date,
    COUNT(*) as total_executions,
    COUNT(*) FILTER (WHERE er.success = true) as successful_executions,
    ROUND(AVG(er.actual_profit_bps), 2) as avg_profit_bps,
    SUM(er.actual_profit_bps) FILTER (WHERE er.success = true) as total_profit_bps,
    ROUND(AVG(er.execution_time_us)::numeric / 1000, 2) as avg_execution_time_ms,
    COUNT(DISTINCT er.wallet_id) as active_wallets
FROM execution_results er
WHERE er.created_at >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE(er.created_at)
ORDER BY trading_date DESC;

-- Wallet performance view
CREATE VIEW wallet_performance AS
SELECT 
    w.id,
    w.pubkey,
    w.tier,
    w.daily_transactions,
    w.total_volume_lamports,
    COUNT(er.id) as total_executions,
    COUNT(er.id) FILTER (WHERE er.success = true) as successful_executions,
    ROUND(AVG(er.actual_profit_bps), 2) as avg_profit_bps,
    SUM(er.actual_profit_bps) FILTER (WHERE er.success = true) as total_profit_bps
FROM wallets w
LEFT JOIN execution_results er ON w.id = er.wallet_id
WHERE w.is_active = true
GROUP BY w.id, w.pubkey, w.tier, w.daily_transactions, w.total_volume_lamports;

-- System health view
CREATE VIEW system_health AS
SELECT 
    'arbitrage_opportunities' as metric_name,
    COUNT(*) as value,
    'last_hour' as time_period
FROM arbitrage_opportunities 
WHERE created_at >= NOW() - INTERVAL '1 hour'
UNION ALL
SELECT 
    'successful_executions' as metric_name,
    COUNT(*) as value,
    'last_hour' as time_period
FROM execution_results 
WHERE created_at >= NOW() - INTERVAL '1 hour' AND success = true
UNION ALL
SELECT 
    'active_wallets' as metric_name,
    COUNT(*) as value,
    'current' as time_period
FROM wallets 
WHERE is_active = true;

-- ==============================================================================
-- COMMENTS AND DOCUMENTATION
-- ==============================================================================

COMMENT ON TABLE wallets IS 'Secure storage for trading wallets with encrypted private keys';
COMMENT ON TABLE arbitrage_opportunities IS 'Detected arbitrage opportunities across different venues';
COMMENT ON TABLE execution_results IS 'Results of arbitrage trade executions';
COMMENT ON TABLE performance_metrics IS 'System performance metrics aggregated over time';
COMMENT ON TABLE dead_letter_queue IS 'Failed events queue for retry processing';
COMMENT ON TABLE audit_log IS 'Security and operational audit trail';

-- ==============================================================================
-- REPORTING VIEWS
-- ==============================================================================

CREATE OR REPLACE VIEW realtime_preflight_metrics AS
SELECT
    ao.timestamp,
    ao.opportunity_id,
    ao.token_pair,
    ao.venue_sell,
    ao.venue_buy,
    ao.profit_bps,
    ao.amount_in,
    ao.expected_amount_out,
    er.status,
    er.estimated_profit_lamports,
    er.estimated_bundle_fees_lamports,
    er.estimated_priority_fee_lamports,
    er.estimated_cu_cost_lamports,
    er.flash_loan_fee_lamports,
    er.min_profit_threshold_lamports,
    er.error_message,
    er.executed_at,
    er.simulated_token_deltas,
    (COALESCE(er.estimated_bundle_fees_lamports, 0)
     + COALESCE(er.estimated_priority_fee_lamports, 0)
     + COALESCE(er.estimated_cu_cost_lamports, 0)
     + COALESCE(er.flash_loan_fee_lamports, 0)) AS total_estimated_cost_lamports,
    (COALESCE(er.estimated_profit_lamports, 0)::numeric / 1e9) AS estimated_profit_sol,
    (COALESCE(er.min_profit_threshold_lamports, 0)::numeric / 1e9) AS min_profit_threshold_sol
FROM arbitrage_opportunities ao
LEFT JOIN LATERAL (
    SELECT er_inner.*
    FROM execution_results er_inner
    WHERE er_inner.opportunity_id::text = ao.opportunity_id
    ORDER BY er_inner.executed_at DESC
    LIMIT 1
) er ON TRUE;

CREATE OR REPLACE VIEW preflight_hourly_summary AS
SELECT
    date_trunc('hour', er.executed_at) AS hour_bucket,
    COUNT(*) FILTER (WHERE er.status = 'PreflightSuccess') AS preflight_passes,
    COUNT(*) FILTER (WHERE er.status <> 'PreflightSuccess') AS preflight_rejections,
    AVG(er.estimated_profit_lamports)::numeric / 1e9 AS avg_profit_sol,
    AVG((COALESCE(er.estimated_bundle_fees_lamports, 0)
        + COALESCE(er.estimated_priority_fee_lamports, 0)
        + COALESCE(er.estimated_cu_cost_lamports, 0)
        + COALESCE(er.flash_loan_fee_lamports, 0)))::numeric / 1e9 AS avg_cost_sol,
    AVG((er.estimated_profit_lamports
        - COALESCE(er.estimated_bundle_fees_lamports, 0)
        - COALESCE(er.estimated_priority_fee_lamports, 0)
        - COALESCE(er.estimated_cu_cost_lamports, 0)
        - COALESCE(er.flash_loan_fee_lamports, 0)))::numeric / 1e9 AS avg_net_sol
FROM execution_results er
WHERE er.executed_at >= NOW() - INTERVAL '48 hours'
GROUP BY hour_bucket
ORDER BY hour_bucket DESC;

CREATE OR REPLACE VIEW preflight_route_breakdown AS
SELECT
    ao.token_pair,
    ao.venue_sell || '→' || ao.venue_buy AS route,
    COUNT(*) FILTER (WHERE er.status = 'PreflightSuccess') AS passes,
    COUNT(*) FILTER (WHERE er.status <> 'PreflightSuccess') AS rejections,
    AVG((er.estimated_profit_lamports
        - COALESCE(er.estimated_bundle_fees_lamports, 0)
        - COALESCE(er.estimated_priority_fee_lamports, 0)
        - COALESCE(er.estimated_cu_cost_lamports, 0)
        - COALESCE(er.flash_loan_fee_lamports, 0)))::bigint AS avg_net_lamports,
    AVG(er.estimated_cu_cost_lamports)::bigint AS avg_cu_cost_lamports,
    AVG(er.estimated_priority_fee_lamports)::bigint AS avg_priority_fee_lamports
FROM arbitrage_opportunities ao
JOIN execution_results er ON er.opportunity_id::text = ao.opportunity_id
WHERE er.executed_at >= NOW() - INTERVAL '48 hours'
GROUP BY ao.token_pair, route
ORDER BY passes DESC, avg_net_lamports DESC;

-- ==============================================================================
-- GRANT PERMISSIONS
-- ==============================================================================

-- Create application user
-- CREATE USER mevana_app WITH PASSWORD 'secure_password_here';

-- Grant permissions (uncomment when creating application user)
-- GRANT CONNECT ON DATABASE mevana TO mevana_app;
-- GRANT USAGE ON SCHEMA public TO mevana_app;
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO mevana_app;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO mevana_app;

-- Create read-only user for reporting
-- CREATE USER mevana_readonly WITH PASSWORD 'readonly_password_here';
-- GRANT CONNECT ON DATABASE mevana TO mevana_readonly;
-- GRANT USAGE ON SCHEMA public TO mevana_readonly;
-- GRANT SELECT ON ALL TABLES IN SCHEMA public TO mevana_readonly;

-- ==============================================================================
-- COMPLETION MESSAGE
-- ==============================================================================

SELECT 'MEVana database schema created successfully!' as status;
