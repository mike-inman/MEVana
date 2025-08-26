-- Raw Transaction Logger Table
-- Stores all incoming transactions with DEX identification

CREATE TABLE IF NOT EXISTS raw_dex_transactions (
    id SERIAL PRIMARY KEY,
    transaction_signature VARCHAR(88) NOT NULL,
    program_id VARCHAR(44) NOT NULL,
    dex_name VARCHAR(50),
    slot_number BIGINT,
    block_time TIMESTAMP,
    raw_message JSONB NOT NULL,
    message_size INTEGER,
    instruction_count INTEGER,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Create indexes separately
CREATE INDEX IF NOT EXISTS idx_program_id ON raw_dex_transactions(program_id);
CREATE INDEX IF NOT EXISTS idx_dex_name ON raw_dex_transactions(dex_name);
CREATE INDEX IF NOT EXISTS idx_transaction_signature ON raw_dex_transactions(transaction_signature);
CREATE INDEX IF NOT EXISTS idx_block_time ON raw_dex_transactions(block_time);
CREATE INDEX IF NOT EXISTS idx_created_at ON raw_dex_transactions(created_at);

-- Known DEX program IDs for classification
CREATE TABLE IF NOT EXISTS dex_program_registry (
    program_id VARCHAR(44) PRIMARY KEY,
    dex_name VARCHAR(50) NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Insert known DEX program IDs
INSERT INTO dex_program_registry (program_id, dex_name, description) VALUES
('JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB', 'Jupiter', 'Jupiter V6 Aggregator'),
('whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc', 'Orca', 'Orca Whirlpool DEX'),
('675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8', 'Raydium', 'Raydium AMM'),
('CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK', 'Raydium', 'Raydium CLMM'),
('Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB', 'Meteora', 'Meteora Dynamic AMM'),
('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', 'Pump.fun', 'Pump.fun AMM'),
('PhoeNiX7VLdNVFSxJvF6fC6UrCsAAu8dffMJJyVhJKEi', 'Phoenix', 'Phoenix Order Book')
ON CONFLICT (program_id) DO NOTHING;
