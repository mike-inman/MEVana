-- Enable pgcrypto for encryption (idempotent)
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Run provenance table
CREATE TABLE IF NOT EXISTS run_provenance (
    run_id UUID PRIMARY KEY,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    git_commit TEXT,
    config_hash TEXT NOT NULL,
    env_redacted JSONB NOT NULL,
    version TEXT,
    notes TEXT
);
CREATE INDEX IF NOT EXISTS idx_run_provenance_started_at ON run_provenance(started_at);

-- Example sensitive secret store (optional)
CREATE TABLE IF NOT EXISTS encrypted_secrets (
    key TEXT PRIMARY KEY,
    value_bytea BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
