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
    created_at TIMESTAMP DEFAULT NOW(),
    
    -- Indexes for fast queries
    INDEX idx_program_id (program_id),
    INDEX idx_dex_name (dex_name),
    INDEX idx_transaction_signature (transaction_signature),
    INDEX idx_block_time (block_time),
    INDEX idx_created_at (created_at)
);

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

-- View for easy querying
CREATE OR REPLACE VIEW dex_transaction_summary AS
SELECT 
    rd.dex_name,
    rd.program_id,
    COUNT(*) as transaction_count,
    MIN(rd.created_at) as first_seen,
    MAX(rd.created_at) as last_seen,
    AVG(rd.message_size) as avg_message_size,
    AVG(rd.instruction_count) as avg_instruction_count
FROM raw_dex_transactions rd
GROUP BY rd.dex_name, rd.program_id
ORDER BY transaction_count DESC;
