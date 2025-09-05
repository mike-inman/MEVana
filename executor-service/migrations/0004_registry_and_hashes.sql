-- Add config & registry hash columns to execution_results (idempotent)
ALTER TABLE execution_results ADD COLUMN IF NOT EXISTS config_hash TEXT;
ALTER TABLE execution_results ADD COLUMN IF NOT EXISTS token_registry_hash TEXT;

-- Basic token registry persistence tables
CREATE TABLE IF NOT EXISTS token_registry_tokens (
    mint TEXT PRIMARY KEY,
    decimals INT,
    symbol TEXT,
    name TEXT,
    first_seen TIMESTAMPTZ DEFAULT NOW(),
    last_refreshed TIMESTAMPTZ DEFAULT NOW(),
    tags TEXT[]
);
CREATE TABLE IF NOT EXISTS token_oracles (
    mint TEXT PRIMARY KEY,
    price_lamports NUMERIC,
    price_decimals INT,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_token_registry_tokens_last_refreshed ON token_registry_tokens(last_refreshed);
CREATE INDEX IF NOT EXISTS idx_token_oracles_updated_at ON token_oracles(updated_at);