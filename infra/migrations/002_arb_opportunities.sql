-- Create arb_opportunities table for storing arbitrage opportunities
CREATE TABLE IF NOT EXISTS arb_opportunities (
    id SERIAL PRIMARY KEY,
    source_dex TEXT NOT NULL,
    target_dex TEXT NOT NULL,
    token_pair TEXT NOT NULL,
    profit_estimate DOUBLE PRECISION NOT NULL,
    raw_data JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Create index for faster queries
CREATE INDEX IF NOT EXISTS idx_arb_opportunities_token_pair ON arb_opportunities(token_pair);
CREATE INDEX IF NOT EXISTS idx_arb_opportunities_created_at ON arb_opportunities(created_at); 