# Operations Guide – October 9, 2025

## Completed Actions
- **Wallet Metrics Exporter**
  - Added Helius + Coingecko price fallbacks with 60s cache to keep SOL/USDC USD values stable.
  - Corrected USDC mint metadata and aggregation so balances match Phantom (≈20.50 USDC).
  - Added warning log for missing token metadata and exposed `HELIUS_API_KEY` in `/etc/mevana/wallet-metrics.env`.
- **Executor Service**
  - Token balance fetch now sums across all SPL accounts for the same mint (covers token accounts created by Phantom).
- **Grafana Dashboard (mevana-live-test)**
  - New “Current USDC Balance” stat card; SOL and USDC totals display in USD.
  - Smoothed “Wallet Value USD (Over Time)” with 2m max to avoid zero spikes; total stat uses 5m last value.
  - Token Balances table now shows one row per token (SOL + USDC) with clean column names.
  - Dashboard JSON synced to server (`/opt/MEVana/infrastructure/grafana/dashboards/curated/mevana-live-test-monitor.json`) and Grafana reloaded.

## Verification Steps
1. `curl http://localhost:9905/metrics | grep mevana_wallet_value_usd` → expect ~64.28.
2. `curl http://localhost:9905/metrics | grep mevana_wallet_token_balance` → SOL ≈0.1999, USDC ≈20.506971.
3. `curl http://localhost:9090/api/v1/targets` → wallet-metrics job `health":"up"`.
4. Grafana: confirm SOL/USDC cards and token table rows, wallet value chart no longer drops to 0.

## Next Focus
- Build Grafana visuals for opportunity detection and preflight checks (table + chart) without enabling live trading.
- Ensure preflight pipeline captures full details (slippage, gas, net profit) and reports them in dashboards.

