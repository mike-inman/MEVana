-- Reconciled execution PnL snapshots (pre/post wallet + token deltas)
CREATE TABLE IF NOT EXISTS execution_reconciliation (
    execution_id UUID PRIMARY KEY REFERENCES execution_results(id) ON DELETE CASCADE,
    wallet_address TEXT NOT NULL,
    pre_sol_lamports BIGINT,
    post_sol_lamports BIGINT,
    sol_delta_lamports BIGINT,
    token_deltas JSONB,               -- { mint: delta_lamports_estimate }
    slot BIGINT,                      -- slot at which post snapshot taken
    finalized BOOLEAN NOT NULL DEFAULT false,  -- upgraded when extra confirmations reached
    divergence_lamports BIGINT,       -- (estimated - reconciled)
    divergence_ratio DOUBLE PRECISION, -- reconciled / estimated (nullable if est=0)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_execution_reconciliation_slot ON execution_reconciliation(slot);
CREATE INDEX IF NOT EXISTS idx_execution_reconciliation_finalized ON execution_reconciliation(finalized);

-- Optional trigger to bump updated_at
CREATE OR REPLACE FUNCTION touch_execution_reconciliation() RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END; $$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_touch_execution_reconciliation ON execution_reconciliation;
CREATE TRIGGER trg_touch_execution_reconciliation
BEFORE UPDATE ON execution_reconciliation
FOR EACH ROW EXECUTE FUNCTION touch_execution_reconciliation();