# MAD MAX MEV Orderbook History System Plan

## 🎯 Overview
Comprehensive PostgreSQL-powered orderbook and arbitrage tracking system for the MAD MAX MEV engine.

## 📊 Database Schema Design

### 1. `orderbook_snapshots` - Real-time bid/ask data
```sql
CREATE TABLE orderbook_snapshots (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    slot BIGINT NOT NULL,
    venue VARCHAR(50) NOT NULL,
    market VARCHAR(100) NOT NULL,
    best_bid DECIMAL(20,8),
    best_ask DECIMAL(20,8),
    mid_price DECIMAL(20,8),
    spread_bps INTEGER,
    bid_depth_usd DECIMAL(15,2),
    ask_depth_usd DECIMAL(15,2),
    total_bids INTEGER,
    total_asks INTEGER,
    raw_orderbook JSONB -- Full orderbook data
);

-- Indexes for performance
CREATE INDEX idx_orderbook_timestamp ON orderbook_snapshots(timestamp);
CREATE INDEX idx_orderbook_venue_market ON orderbook_snapshots(venue, market);
CREATE INDEX idx_orderbook_slot ON orderbook_snapshots(slot);
```

### 2. `arbitrage_opportunities` - Every detected opportunity
```sql
CREATE TABLE arbitrage_opportunities (
    id BIGSERIAL PRIMARY KEY,
    opportunity_id VARCHAR(50) UNIQUE NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    slot BIGINT NOT NULL,
    venue_sell VARCHAR(50) NOT NULL,
    venue_buy VARCHAR(50) NOT NULL,
    token_pair VARCHAR(100) NOT NULL,
    profit_bps INTEGER NOT NULL,
    amount_in DECIMAL(20,8) NOT NULL,
    expected_amount_out DECIMAL(20,8) NOT NULL,
    price_impact_pct DECIMAL(8,4),
    detection_time_us BIGINT, -- Time to detect opportunity
    strategy_type VARCHAR(50), -- 'spread-arbitrage', 'impact-arb', etc.
    raw_opportunity JSONB -- Full opportunity data
);

-- Indexes
CREATE INDEX idx_arbitrage_timestamp ON arbitrage_opportunities(timestamp);
CREATE INDEX idx_arbitrage_venues ON arbitrage_opportunities(venue_sell, venue_buy);
CREATE INDEX idx_arbitrage_profit ON arbitrage_opportunities(profit_bps);
```

### 3. `execution_results` - Success/failure tracking
```sql
CREATE TABLE execution_results (
    id BIGSERIAL PRIMARY KEY,
    opportunity_id VARCHAR(50) NOT NULL REFERENCES arbitrage_opportunities(opportunity_id),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    executed BOOLEAN NOT NULL,
    success BOOLEAN,
    actual_profit_bps INTEGER,
    execution_time_us BIGINT,
    gas_cost_sol DECIMAL(10,8),
    slippage_bps INTEGER,
    failure_reason TEXT,
    transaction_signature VARCHAR(100),
    bundle_id VARCHAR(100),
    raw_result JSONB
);

-- Indexes
CREATE INDEX idx_execution_timestamp ON execution_results(timestamp);
CREATE INDEX idx_execution_success ON execution_results(success);
CREATE INDEX idx_execution_opportunity ON execution_results(opportunity_id);
```

### 4. `venue_performance` - Aggregated statistics
```sql
CREATE TABLE venue_performance (
    id BIGSERIAL PRIMARY KEY,
    venue_pair VARCHAR(100) NOT NULL, -- 'meteora->orca'
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    total_opportunities INTEGER DEFAULT 0,
    executed_opportunities INTEGER DEFAULT 0,
    successful_executions INTEGER DEFAULT 0,
    total_profit_bps BIGINT DEFAULT 0,
    average_profit_bps DECIMAL(8,2),
    average_execution_time_us DECIMAL(10,2),
    success_rate_pct DECIMAL(5,2),
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(venue_pair, date)
);

-- Indexes
CREATE INDEX idx_venue_performance_date ON venue_performance(date);
CREATE INDEX idx_venue_performance_pair ON venue_performance(venue_pair);
```

## 🚀 **ZERO-LATENCY HOTPATH DESIGN**

### **Critical Performance Requirements:**
- ✅ **NO database writes on hotpath** - All arbitrage detection stays sub-50μs
- ✅ **Async circular buffer** - Non-blocking sends to background worker
- ✅ **Batch writes** - Efficient PostgreSQL bulk inserts every 5 seconds
- ✅ **Error recovery** - No data loss even if DB temporarily unavailable

### **Hotpath Recording (< 1μs overhead):**
```rust
// This is ALL that happens on the hotpath - ultra-fast inline functions
history_buffer.record_opportunity(create_arbitrage_opportunity(
    &opportunity_id, slot, timestamp_ns, "meteora", "orca", 
    "SOL/USDC", profit_bps, amount_in, amount_out, detection_time_us,
    "impact-arb", raw_json
)); // Returns immediately, no blocking
```

### **Background Worker:**
- **Circular buffer**: 10,000 events before auto-flush
- **Batch processing**: 1,000 events per transaction  
- **Auto-flush timer**: Every 5 seconds
- **Memory efficient**: Drops oldest events if buffer overflows

## ⚡ Implementation Tasks

### Phase 1: Database Setup (5 minutes)
- [ ] Create database schema in PostgreSQL
- [ ] Add indexes for optimal query performance
- [ ] Test database connectivity

### Phase 2: Integration (15 minutes)  
- [ ] Add PostgreSQL client to hotpath_arb_engine
- [ ] Implement orderbook snapshot recording
- [ ] Implement arbitrage opportunity logging
- [ ] Implement execution result tracking

### Phase 3: Query Interface (10 minutes)
- [ ] Add HTTP endpoints for history queries
- [ ] Create real-time orderbook endpoint
- [ ] Create performance statistics endpoint
- [ ] Add filtering and pagination

### Phase 4: Deployment (5 minutes)
- [ ] Deploy updated hotpath_arb_engine
- [ ] Test database integration
- [ ] Verify data collection

## 🎯 Expected Outcomes

### Immediate Visibility:
- Real-time orderbook state across all venues
- Complete arbitrage opportunity timeline
- Success/failure analysis with reasons
- Venue performance comparison

### Query Examples:
```sql
-- Recent profitable opportunities
SELECT * FROM arbitrage_opportunities 
WHERE profit_bps > 10 
ORDER BY timestamp DESC LIMIT 50;

-- Venue success rates
SELECT venue_pair, success_rate_pct, average_profit_bps 
FROM venue_performance 
WHERE date = CURRENT_DATE 
ORDER BY success_rate_pct DESC;

-- Orderbook spread analysis
SELECT venue, market, AVG(spread_bps) as avg_spread
FROM orderbook_snapshots 
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY venue, market
ORDER BY avg_spread;
```

## 🔧 Environment Configuration

Database connection for hotpath_arb_engine:
```bash
export DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb"
export ORDERBOOK_HISTORY_ENABLED=true
export ORDERBOOK_SNAPSHOT_INTERVAL_MS=1000
```

## 📊 HTTP Endpoints

- `GET /orderbook/live` - Current orderbook state
- `GET /orderbook/history?venue=&market=&limit=` - Historical snapshots  
- `GET /arbitrage/opportunities?limit=&venue_pair=` - Opportunity history
- `GET /arbitrage/performance` - Venue performance stats
- `GET /stats/summary` - Overall system statistics

This system provides complete visibility into your MAD MAX MEV engine's decision-making process!