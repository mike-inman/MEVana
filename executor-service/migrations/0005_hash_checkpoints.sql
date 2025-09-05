CREATE TABLE IF NOT EXISTS hash_chain_checkpoints (
    id BIGSERIAL PRIMARY KEY,
    checkpoint_time TIMESTAMPTZ DEFAULT NOW(),
    chain_hash TEXT NOT NULL,
    lines_aggregated INT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_hash_chain_checkpoints_time ON hash_chain_checkpoints(checkpoint_time DESC);