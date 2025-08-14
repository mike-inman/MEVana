
CREATE TABLE IF NOT EXISTS arbitrage_signals (
    id SERIAL PRIMARY KEY,
    pair TEXT NOT NULL,
    buy_on TEXT NOT NULL,
    sell_on TEXT NOT NULL,
    price_diff DOUBLE PRECISION NOT NULL,
    expected_profit DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS arbitrage_executions (
    id SERIAL PRIMARY KEY,
    signal_id INT REFERENCES arbitrage_signals(id),
    tx_signature TEXT NOT NULL,
    status TEXT NOT NULL,
    profit_estimate DOUBLE PRECISION NOT NULL,
    executed_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS arbitrage_opportunities (
    id SERIAL PRIMARY KEY,
    base_token TEXT NOT NULL,
    quote_token TEXT NOT NULL,
    buy_dex TEXT NOT NULL,
    sell_dex TEXT NOT NULL,
    est_profit DOUBLE PRECISION NOT NULL,
    slot BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);
