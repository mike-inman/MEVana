CREATE TABLE arbitrage_opportunities (
    id SERIAL PRIMARY KEY,
    dex_a VARCHAR(50),
    dex_b VARCHAR(50),
    token_pair VARCHAR(50),
    price_a NUMERIC,
    price_b NUMERIC,
    spread NUMERIC,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE executions (
    id SERIAL PRIMARY KEY,
    opportunity_id INT REFERENCES arbitrage_opportunities(id),
    tx_signature VARCHAR(100),
    status VARCHAR(20),
    profit_estimate NUMERIC,
    executed_at TIMESTAMP DEFAULT NOW()
);
