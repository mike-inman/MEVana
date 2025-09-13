-- Executions & PnL tracking initial schema
CREATE TABLE IF NOT EXISTS execution_results (
    id UUID PRIMARY KEY,
    opportunity_id UUID NOT NULL,
    status TEXT NOT NULL,
    bundle_id TEXT,
    transaction_signatures TEXT[] NOT NULL DEFAULT '{}',
    estimated_profit_lamports BIGINT NOT NULL,
    actual_profit_lamports BIGINT,
    execution_time_ms BIGINT NOT NULL,
    gas_used INTEGER,
    error_message TEXT,
    executed_at TIMESTAMPTZ NOT NULL,
    flash_loan_provider TEXT,
    flash_loan_fee_lamports BIGINT,
    estimated_bundle_fees_lamports BIGINT,
    estimated_priority_fee_lamports BIGINT,
    estimated_cu_cost_lamports BIGINT,
    min_profit_threshold_lamports BIGINT,
    simulated_token_deltas JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_execution_results_opportunity ON execution_results(opportunity_id);
CREATE INDEX IF NOT EXISTS idx_execution_results_status ON execution_results(status);
CREATE INDEX IF NOT EXISTS idx_execution_results_executed_at ON execution_results(executed_at);

-- Daily PnL snapshot table (one row per utc day)
CREATE TABLE IF NOT EXISTS daily_pnl (
    day_start DATE PRIMARY KEY,
    realized_pnl_lamports BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
