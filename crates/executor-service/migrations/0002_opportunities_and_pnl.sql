-- Store raw opportunities and ensure daily pnl upsert helper
CREATE TABLE IF NOT EXISTS opportunities (
    id UUID PRIMARY KEY,
    token_in TEXT NOT NULL,
    token_out TEXT NOT NULL,
    profit_bps BIGINT NOT NULL,
    max_size_in NUMERIC(38,0) NOT NULL,
    total_cu_estimate INTEGER,
    total_fees_bps INTEGER,
    detected_at TIMESTAMPTZ NOT NULL,
    max_edge_age_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_opportunities_detected_at ON opportunities(detected_at);

-- Helper function for updating daily_pnl
CREATE OR REPLACE FUNCTION upsert_daily_pnl(p_day date, p_delta BIGINT) RETURNS VOID AS $$
BEGIN
    LOOP
        UPDATE daily_pnl SET realized_pnl_lamports = realized_pnl_lamports + p_delta, updated_at = NOW() WHERE day_start = p_day;
        IF FOUND THEN RETURN; END IF;
        BEGIN
            INSERT INTO daily_pnl(day_start, realized_pnl_lamports) VALUES (p_day, p_delta);
            RETURN;
        EXCEPTION WHEN unique_violation THEN
        END;
    END LOOP;
END;
$$ LANGUAGE plpgsql;
