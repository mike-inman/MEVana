-- Additional tables required for hotpath_arb_engine
-- Date: 2025-09-09

-- 1. Flash loan usage tracking
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
    user_id UUID REFERENCES users(id), -- For multi-trader support
    
    -- Indexes
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_flash_timestamp ON flash_loan_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_flash_provider ON flash_loan_history(provider);
CREATE INDEX IF NOT EXISTS idx_flash_user ON flash_loan_history(user_id);

-- 2. Performance metrics aggregation
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
    user_id UUID REFERENCES users(id), -- For multi-trader support
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_perf_timestamp ON performance_metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_perf_user ON performance_metrics(user_id);

-- 3. AI strategy optimization history
CREATE TABLE IF NOT EXISTS ai_optimization_history (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    strategy_type VARCHAR(50) NOT NULL,
    parameters JSONB NOT NULL,
    performance_score DOUBLE PRECISION,
    profit_improvement_pct DOUBLE PRECISION,
    applied BOOLEAN DEFAULT false,
    user_id UUID REFERENCES users(id), -- For multi-trader support
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ai_timestamp ON ai_optimization_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_ai_strategy ON ai_optimization_history(strategy_type);
CREATE INDEX IF NOT EXISTS idx_ai_user ON ai_optimization_history(user_id);

-- Note: The 'wallets' table already exists and will be used via wallet-vault service
-- The hotpath_arb_engine will:
-- - Use wallet-vault API for wallet management operations (off-hotpath)
-- - Cache wallet pubkeys in memory for fast trading execution (on-hotpath)

-- Add user_id to existing tables for multi-trader support (future enhancement)
-- These are commented out for now but documented for future implementation:
-- ALTER TABLE arbitrage_opportunities ADD COLUMN IF NOT EXISTS user_id UUID REFERENCES users(id);
-- ALTER TABLE execution_results ADD COLUMN IF NOT EXISTS user_id UUID REFERENCES users(id);
-- ALTER TABLE orderbook_snapshots ADD COLUMN IF NOT EXISTS user_id UUID REFERENCES users(id);