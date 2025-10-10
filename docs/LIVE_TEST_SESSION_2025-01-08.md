# MEVana Live Test Trading Session - January 8, 2025

**Status:** In Progress - Preparing for preflight-only test
**Server:** 149.28.56.125 (root access)
**Branch:** restore-20250930
**Context:** Building monitoring dashboard before first live trade test

---

## 🎯 **Session Goals**

### Primary Objectives (24-48 hours)
1. **Execute REAL live test trade** - Verify end-to-end execution with actual crypto
2. **Set up Slack alerts** - Get notifications when arb opportunities detected + preflight results
3. **Display wallet balance in USD** - Real-time monitoring in Grafana
4. **Display arbitrage opportunities table** - Tabular view of all opportunities in Grafana

### Current Focus
**Building preflight-only monitoring system** to test arbitrage detection without risk

---

## 📊 **Progress Summary**

### ✅ Completed
1. **Server access verified** - SSH to 149.28.56.125 as root working
2. **Critical infrastructure issue fixed:**
   - Server was 100% disk full (466GB/480GB used)
   - Root cause: Kafka had no named volume → created 241GB orphaned volume
   - **Fixed:** Added `kafka_data` named volume to docker-compose files
   - **Fixed:** Added `KAFKA_LOG_RETENTION_BYTES=10737418240` (10GB limit)
   - **Created:** `scripts/maintenance/docker-cleanup.sh` for automated cleanup
   - **Result:** Freed 300GB, now at 37% disk usage (166GB/480GB)
   - **Committed:** fbc10431 "fix: add Kafka named volume and cleanup script"

3. **Docker infrastructure healthy:**
   - Postgres: Running (mevana_postgres_data volume)
   - Kafka: Running (new kafka_data volume)
   - Redis: Running
   - Grafana: Running on port 3000
   - Prometheus: Running on port 9090

### 🔄 In Progress
1. Building wallet balance metrics exporter
2. Creating Grafana "Live Test Monitor" dashboard
3. Configuring MEVana for preflight-only mode

### ⏳ Pending
1. Fund Phantom wallet (user is preparing)
2. Transfer wallet keypair to server
3. Deploy and run preflight tests
4. Execute live trade if preflight validates

---

## 🧪 **Test Strategy (REVISED - Better Approach)**

### Original Plan (Rejected)
- Manual USDC → USDT swap on Orca
- Claude builds transaction manually
- Simple but doesn't test real system

### **APPROVED Plan: Let MEVana Do Everything**

**Philosophy:** Test the ACTUAL arbitrage system, not a manual transaction

**Configuration:**
```bash
# Safety Mode - Preflight Only
PAPER_TRADING=true
DRY_RUN=true
ENABLE_LIVE_EXECUTION=false
PREFLIGHT_ALL_OPPORTUNITIES=true
STOP_AFTER_PREFLIGHT=true              # NEW - stops before blockchain submission

# Position Sizing (Tight for Safety)
MIN_POSITION_SIZE_SOL=0.005            # $1 minimum (~0.005 SOL)
MAX_POSITION_SIZE_SOL=0.02             # $4 maximum (~0.02 SOL)

# Profit Threshold (Very Low to See Opportunities)
MIN_PROFIT_THRESHOLD_BPS=1             # 0.01% - any profit after costs

# Slippage
MAX_SLIPPAGE_BPS=50                    # 0.5% max

# Token Universe (Expanded for Real Arbitrage)
ARBITRAGE_TOKENS="SOL,USDC,BONK,JUP,WIF,JTO,PYTH,ORCA"
# Why: USDC/USDT rarely has arbitrage (both pegged to $1)
#      These tokens have price differences across DEXes

# Venues
ENABLED_VENUES="orca,raydium,meteora"
```

**Execution Flow:**
1. Start HotPath engine in preflight-only mode
2. System detects real arbitrage opportunities
3. System assembles swap transactions
4. System runs preflight simulation
5. System logs results to Prometheus + PostgreSQL
6. **STOPS** - does not execute on blockchain
7. We watch in Grafana dashboard

**Expected Results (30 min run):**
- Opportunities detected: 5-15
- Most common: SOL/USDC cross-venue, BONK arbitrage
- Typical profit: 5-20 BPS gross, 2-15 BPS net after costs
- Preflight success rate: 80-90%

**What This Tests:**
- ✅ Arbitrage detection (does it find real opportunities?)
- ✅ Transaction assembly (can it build valid swaps?)
- ✅ Cost estimation (gas + slippage + fees)
- ✅ Preflight simulation (would they succeed?)
- ✅ Data collection (monitoring/logging works?)
- ✅ Safety limits (MAX_POSITION_SIZE enforced?)

**Why This Is Better:**
- Tests real system end-to-end
- Zero financial risk (preflight only)
- Real market data, real opportunities
- Full visibility in Grafana
- Can iterate without losing money

---

## 📊 **Grafana Dashboard Design**

### Dashboard: "MEVana Live Test Monitor"

**Panels to Build:**

#### Row 1: Wallet Status (Real-time)
- **Panel:** Current SOL Balance
  - Metric: `wallet_balance_sol{wallet="hot"}`
  - Refresh: 5 seconds

- **Panel:** Current USDC Balance
  - Metric: `wallet_token_balance{wallet="hot",token="USDC"}`

- **Panel:** Current Token Balances (Table)
  - All SPL tokens with non-zero balance

- **Panel:** Wallet Value USD
  - Calculated: balances * prices

#### Row 2: Opportunity Detection
- **Panel:** Opportunities Detected (Counter)
  - Metric: `arbitrage_opportunities_detected_total`

- **Panel:** Last Opportunity (Stat)
  - Profit BPS, token pair, venues

- **Panel:** Opportunity Rate (Graph)
  - Opportunities per minute over time

#### Row 3: Preflight Results
- **Panel:** Preflight Simulations Run (Counter)
  - Metric: `preflight_simulations_total`

- **Panel:** Preflight Success Rate (Gauge)
  - Success / Total * 100

- **Panel:** Failed Preflights (Table)
  - Reasons for failure

- **Panel:** Compute Units Estimated (Graph)
  - Average CU per opportunity

#### Row 4: Would-Be Execution (If We Had Executed)
- **Panel:** Expected Profit (Stat)
  - Sum of net profit from all valid preflights

- **Panel:** Expected Slippage (Graph)
  - Estimated slippage per opportunity

- **Panel:** Estimated Gas Costs (Counter)
  - Total gas if we had executed

- **Panel:** Net Profit After Fees (Stat)
  - Gross - Gas - Slippage - DEX Fees

#### Row 5: System Health
- **Panel:** Services Running (Status)
  - HotPath, Executor, Normalizer, Discovery

- **Panel:** Geyser Connection (Stat)
  - Connected/Disconnected, slot lag

- **Panel:** Database Writes (Counter)
  - Opportunities written to PostgreSQL

---

## 🛠️ **Implementation Plan**

### Phase 1: Wallet Balance Metrics Exporter (NEW SERVICE)

**File:** `crates/wallet-metrics-exporter/src/main.rs`

**Purpose:** Query wallet balances from Shyft API, expose to Prometheus

**Metrics Exposed:**
```
wallet_balance_sol{wallet="hot"} 0.195
wallet_token_balance{wallet="hot",token="USDC"} 20.0
wallet_token_balance{wallet="hot",token="USDT"} 0.0
wallet_token_balance{wallet="hot",token="BONK"} 0.0
sol_price_usd 200.5
wallet_value_usd{wallet="hot"} 59.1
last_balance_update_timestamp 1704729600
```

**Dependencies:**
- Shyft API client (already in executor-service)
- Prometheus metrics exporter
- Token price fetcher (CoinGecko or Jupiter)

**Deployment:**
- Run as systemd service on server
- Expose metrics on port 9905
- Update Prometheus scrape config

### Phase 2: Enhanced HotPath Metrics

**Add to:** `crates/hotpath_arb_engine/src/metrics.rs`

**New Metrics:**
```rust
// Opportunity detection
arbitrage_opportunities_detected_total
arbitrage_opportunity_profit_bps
arbitrage_opportunity_size_sol

// Preflight results
preflight_simulations_total{result="success|failure"}
preflight_simulation_cu_estimate
preflight_failure_reasons{reason="..."}

// Cost estimation
estimated_gas_cost_lamports
estimated_slippage_bps
estimated_dex_fees_bps
net_profit_after_costs_bps
```

**Code Changes Needed:**
1. Add metric declarations to MetricsServer
2. Update opportunity detection to increment counters
3. Update preflight runner to record results
4. Expose on existing port 8088

### Phase 3: Grafana Dashboard JSON

**File:** `infrastructure/grafana/dashboards/curated/mevana-live-test-monitor.json`

**Configuration:**
- Auto-refresh: 5 seconds
- Time range: Last 1 hour
- Variables: `$wallet` (for multi-wallet support)
- Annotations: Mark when services start/stop

**Datasources:**
- Prometheus (metrics)
- PostgreSQL (opportunity history table)

### Phase 4: Environment Configuration

**File:** `/etc/mevana/hotpath.env` (on server)

**Add/Update:**
```bash
# Preflight Mode
PAPER_TRADING=true
DRY_RUN=true
ENABLE_LIVE_EXECUTION=false
PREFLIGHT_ALL_OPPORTUNITIES=true
STOP_AFTER_PREFLIGHT=true

# Position Sizing
MIN_POSITION_SIZE_SOL=0.005
MAX_POSITION_SIZE_SOL=0.02

# Profit/Slippage
MIN_PROFIT_THRESHOLD_BPS=1
MAX_SLIPPAGE_BPS=50

# Token Universe
ARBITRAGE_TOKENS=SOL,USDC,BONK,JUP,WIF,JTO,PYTH,ORCA

# Venues
ENABLED_VENUES=orca,raydium,meteora

# Metrics
METRICS_PORT=8088
WALLET_METRICS_PORT=9905
```

**File:** `/etc/mevana/executor.env` (on server)

**Add:**
```bash
# Wallet path (will be set after transfer)
WALLET_KEYPAIR_PATH=/opt/MEVana/keys/test-wallet.json

# Shyft for balance queries
SHYFT_API_KEY=efff2d23-97ae-4c5f-8b8b-9a6265001189
SHYFT_ENDPOINT=https://grpc.ny.shyft.to
```

### Phase 5: Deployment Sequence

**Step 1: Build & Deploy Monitoring**
```bash
# On local machine
git add crates/wallet-metrics-exporter
git add infrastructure/grafana/dashboards/curated/mevana-live-test-monitor.json
git commit -m "feat: add wallet metrics exporter and live test dashboard"
git push origin restore-20250930

# Trigger GitHub Actions deployment
gh workflow run self-hosted-deploy.yml \
  --ref restore-20250930 \
  --field services="wallet-metrics-exporter"
```

**Step 2: Update Prometheus Config**
```bash
# SSH to server
ssh root@149.28.56.125

# Add wallet-metrics-exporter to Prometheus targets
cat >> /opt/MEVana/infrastructure/prometheus.prod.yml << 'EOF'
  - job_name: 'wallet-metrics'
    static_configs:
      - targets: ['host.docker.internal:9905']
        labels:
          service: 'wallet-metrics-exporter'
EOF

# Reload Prometheus
docker exec mevana-prometheus-1 kill -HUP 1
```

**Step 3: Import Grafana Dashboard**
```bash
# Copy dashboard JSON to Grafana provisioning directory
rsync -av infrastructure/grafana/dashboards/curated/mevana-live-test-monitor.json \
  root@149.28.56.125:/opt/MEVana/infrastructure/grafana/dashboards/curated/

# Restart Grafana to pick up new dashboard
ssh root@149.28.56.125 "docker restart mevana-grafana-1"
```

**Step 4: Fund Phantom Wallet**
```
User action:
1. Fund Phantom with 0.2 SOL
2. Add small amounts of USDC, BONK, JUP (optional, for more test scenarios)
3. Export private key from Phantom
4. Paste in chat
```

**Step 5: Transfer Wallet to Server**
```bash
# On local machine (after receiving private key)
# Convert base58 private key to Solana keypair JSON
# Transfer to server securely
scp test-wallet.json root@149.28.56.125:/opt/MEVana/keys/
ssh root@149.28.56.125 "chmod 600 /opt/MEVana/keys/test-wallet.json"
```

**Step 6: Start Services**
```bash
ssh root@149.28.56.125

# Start wallet metrics exporter
systemctl start wallet-metrics-exporter
systemctl status wallet-metrics-exporter

# Start HotPath in preflight mode
systemctl start hotpath-arb-engine
systemctl status hotpath-arb-engine

# Verify metrics endpoints
curl -s localhost:9905/metrics | head -20
curl -s localhost:8088/metrics | grep arbitrage
```

**Step 7: Monitor in Grafana**
```
Open browser:
http://149.28.56.125:3000

Navigate to:
Dashboards → MEVana → Live Test Monitor

Watch for:
- Wallet balances updating
- Opportunities detected counter incrementing
- Preflight simulations running
- Results logged
```

---

## 🔍 **Monitoring & Validation**

### Success Indicators
✅ Wallet balance metrics updating every 5 seconds
✅ Opportunities detected within 5-10 minutes
✅ Preflight simulations running successfully
✅ Data appearing in Grafana dashboard
✅ Database writes to `arbitrage_opportunities` table
✅ No error spikes in service logs

### What to Watch For
⚠️ No opportunities detected (may need to lower profit threshold)
⚠️ All preflights failing (transaction assembly issue)
⚠️ Wallet balance not updating (metrics exporter problem)
⚠️ Missing data in Grafana (Prometheus scrape issue)

### Debug Commands
```bash
# Check wallet metrics
curl -s localhost:9905/metrics

# Check HotPath metrics
curl -s localhost:8088/metrics

# View HotPath logs
journalctl -u hotpath-arb-engine -f

# View wallet metrics logs
journalctl -u wallet-metrics-exporter -f

# Check database
ssh root@149.28.56.125 "docker exec mevana-postgres-1 psql -U mev -d mevdb -c 'SELECT COUNT(*) FROM arbitrage_opportunities;'"
```

---

## 📁 **Files to Create/Modify**

### New Files
1. `crates/wallet-metrics-exporter/Cargo.toml`
2. `crates/wallet-metrics-exporter/src/main.rs`
3. `infrastructure/grafana/dashboards/curated/mevana-live-test-monitor.json`
4. `infrastructure/systemd/wallet-metrics-exporter.service`

### Modified Files
1. `crates/hotpath_arb_engine/src/metrics.rs` - Add new metrics
2. `crates/hotpath_arb_engine/src/main.rs` - Add STOP_AFTER_PREFLIGHT logic
3. `/etc/mevana/hotpath.env` - Environment variables
4. `/etc/mevana/executor.env` - Wallet path configuration
5. `infrastructure/prometheus.prod.yml` - Add wallet-metrics scrape target

---

## 🎯 **Next Session Plan**

### When Resuming:
1. **Read this document** to understand current state
2. **Check server status:** `ssh root@149.28.56.125 "docker ps && df -h /"`
3. **Verify Git status:** Latest commit should be fbc10431 or newer
4. **Ask user:** "Ready to proceed with building wallet metrics exporter?"

### Implementation Order:
1. Build wallet-metrics-exporter (20 mins)
2. Create Grafana dashboard JSON (20 mins)
3. Update HotPath metrics (15 mins)
4. Deploy to server (10 mins)
5. Get wallet from user (5 mins)
6. Start services and monitor (30-60 mins)
7. Analyze results and decide on live execution

### Decision Point:
After 30-60 minutes of preflight monitoring:
- **If successful:** Flip ENABLE_LIVE_EXECUTION=true for ONE trade
- **If issues:** Debug and iterate without risk
- **Either way:** We have full monitoring in place for Goals #3 and #4

---

## 📋 **Current Server State**

### Infrastructure
- **IP:** 149.28.56.125
- **SSH:** root access configured
- **Disk:** 166GB/480GB used (37%)
- **Docker Containers Running:**
  - mevana-postgres-1 (healthy)
  - mevana-kafka-1 (healthy)
  - mevana-grafana-1 (healthy, port 3000)
  - mevana-prometheus-1 (healthy, port 9090)
  - mevana-redis-1 (healthy)
  - mevana-zookeeper-1 (healthy)

### Services Status
- HotPath: Not running (will start after wallet configured)
- Executor: Not running
- Normalizer: Not running
- Discovery: Not running
- Wallet-metrics-exporter: Not created yet

### Configuration
- Branch: restore-20250930
- Last commit: fbc10431 (Kafka volume fix)
- Env files: /etc/mevana/*.env

---

## 🚨 **Critical Notes**

### Wallet Security
- Private key will be stored in `/opt/MEVana/keys/test-wallet.json`
- File permissions: 600 (read/write owner only)
- Owner: mevana:mevana
- **This is a TEST wallet only** - small amounts only

### Known Issues Fixed
- ✅ Disk space exhaustion (Kafka orphaned volumes)
- ✅ Docker containers restarting

### Remaining Risks
- ⚠️ First time running with real wallet
- ⚠️ Preflight mode not fully tested in production
- ⚠️ Cost estimation accuracy unknown

### Mitigation
- Start with preflight-only (zero risk)
- Tight position limits ($1-4 per trade)
- Full monitoring before execution
- Manual approval before flipping to live

---

## 📞 **Contact Info**

**User Preferences:**
- Careful, methodical approach
- Document everything for learning
- Test with real system, not manual transactions
- Collect maximum data
- Willing to take time to get it right

**Communication Style:**
- Explain technical decisions
- Show configuration before applying
- Confirm before risky operations
- Learn from mistakes by collecting data

---

**END OF SESSION DOCUMENTATION**

Next Claude instance: Read this entire document before responding to user.
User will say "continue from where we left off" or similar - use this doc as context.
