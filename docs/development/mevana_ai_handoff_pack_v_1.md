# MEVana — AI Handoff Pack (v1.2)
Last updated: 2025-09-30

> **Snapshot (2025‑09‑30)**
> - Hotpath (`hotpath_ar b_engine`) now pushes raw geyser transactions to Kafka (`dex-transactions`) via the in-process `DexKafkaPublisher`. Arbitrage detections continue to land in `alerts.arbitrage.opportunities`; live execution remains blocked by the safety gate.
> - `normalizer-service` consumes `dex-transactions` and emits Whirlpool swaps to `orca-swaps.parsed.v2`. Extractors for Raydium/Meteora/Phoenix still return `None`, so downstream discovery/Grafana remain dry beyond Whirlpool.
> - `price-aggregator` service added: consumes `orca-swaps.parsed.v2`, publishes rolling price snapshots to `prices.latest`, and now ships as a managed systemd unit (`price-aggregator.service`).
> - Current blocker: geyser transaction payloads arrive without token balance deltas, so the generated extractors cannot infer swap amounts; fix requires adjusting the geyser subscription to include token balance meta before discovery/UI can light up.
> - Focus for the next helper: restore token balance metadata for geyser transactions, finish swap extractors, restart discovery once data flows, and refresh Grafana/UI metrics for venue throughput.

> Single doc for any assistant (or future you) to resume work fast. Includes current state, how to run, how to verify, and what’s next.

---

## 0) Snapshot — What’s Running / Topics (2025‑09‑30)

**Core services (Rust/systemd):**

- `hotpath-arb-engine` (bundles the old `multi-geyser-streams`): subscribes to Shyft, runs MEV detection, emits raw tx → `dex-transactions`, arbitrage events → `alerts.arbitrage.opportunities`.
- `normalizer-service`: consumes `dex-transactions`, emits Whirlpool swaps → `orca-swaps.parsed.v2` (other venues pending extractor work).
- `discovery-service`: running, waiting on expanded swap output to resume meaningful metrics.
- `ui-gateway` / `ui-web`: dashboards expect parsed swap/price topics; refresh once data flows beyond Whirlpool.

**Kafka topics in play:**

- `dex-transactions` – **active**, raw geyser feed back online.
- `alerts.arbitrage.opportunities` – **active**, MEV detections landing here; trading still blocked by gate.
- `orca-swaps.parsed.v2` – **active for Whirlpool**; offsets moving after normalizer restart.
- Raydium/Meteora/Phoenix parsed topics – **empty** until extractors emit swaps.
- Legacy topics (`normalized-swaps.v1`, `prices.latest`) — not used by the current pipeline.

**Operational notes:**

- Event bus config now logs parse failures and increments Kafka publish metrics (`event_bus_kafka_published_total`).
- Hotpath metrics include `dex_kafka_enqueued_total` / `dex_kafka_published_total` plus the per-stream rate gauges (`multi_geyser_stream_messages_per_minute`). Wire these into Grafana when cycles allow.
- Block times remain `0` because the geyser payloads omit them—plan on enriching via block stream or tolerating the default for now.

---

## 1) One‑Command Dev Runner

**Script:** `infra/scripts/run-dev.sh`

- Loads `` from repo root.
- Echoes the **ENDPOINT** and **X\_TOKEN** it found.
- Creates `prices.latest` (compacted) if missing.
- Starts, in order: `dex-stream-orca`, `normalizer-orca`, `price-aggregator`, `ui-gateway`, `ui-web`.
- Logs stream to `infra/logs/*.log`.

**Usage:**

```bash
./infra/scripts/run-dev.sh
# CTRL+C stops the tmux session or tail terminals if you’re not using tmux.
```

**If you’re not using tmux:** the script backgrounds services and tails logs; see files in `infra/logs/`.

---

## 2) Manual Runbook (per‑service)

Make sure Kafka is running and topics exist (`kafka-topics --list`). Then:

```bash
# Producer (Geyser → Kafka)
ENDPOINT=<geyser-grpc-url> X_TOKEN=<key> \
KAFKA_BROKER=localhost:29092 TOPIC=orca-swaps \
cargo run -p dex-stream-orca

# Normalizer (Orca)
KAFKA_BROKER=localhost:29092 \
INPUT_TOPIC=orca-swaps OUTPUT_TOPIC=orca-swaps.parsed \
DLQ_TOPIC=orca-swaps.dlq GROUP_ID=normalizer-orca-v1 \
cargo run -p normalizer-orca

# Price Aggregator → prices.latest (compacted)
KAFKA_BROKER=localhost:29092 \
IN_TOPIC=orca-swaps.parsed OUT_TOPIC=prices.latest \
GROUP_ID=price-aggregator \
cargo run -p price-aggregator

# UI Gateway (WebSocket bridge)
KAFKA_BROKER=localhost:29092 \
PRICES_TOPIC=prices.latest SWAPS_TOPIC=orca-swaps.parsed \
cargo run -p ui-gateway

# UI Web (Vite dev)
cd crates/ui-web && npm run dev
```

**Create compacted topic once:**

```bash
kafka-topics --bootstrap-server localhost:29092 \
  --create --if-not-exists --topic prices.latest \
  --partitions 3 --replication-factor 1 \
  --config cleanup.policy=compact
```

---

## 3) Verification Checklist (5 mins)

### A) Stream/Normalize path

- Check `orca-swaps.parsed` has data:
  ```bash
  kafka-console-consumer --bootstrap-server localhost:29092 \
    --topic orca-swaps.parsed --from-beginning --max-messages 3
  ```
- If `token_in`/`token_out` are occasionally null, that’s expected (heuristic); codegen upgrade will improve this.

### B) Pricing path

- Check `prices.latest`:
  ```bash
  kafka-console-consumer --bootstrap-server localhost:29092 \
    --topic prices.latest --from-beginning --max-messages 3
  ```

### C) Gateway & UI

- `ui-gateway` logs: “listening on 0.0.0.0:8081” and “consuming topic=…”.
- Open UI: `http://localhost:5173` → header WS pill should turn **green (live)**.
- Price cards should populate; swaps feed should scroll.

### D) Codegen proof (normalizer‑service)

- Build: `cargo clean -p normalizer-service && cargo build -p normalizer-service`
- Expect build messages like **“INFO: Loaded IDLs”** / **“Loaded profiles”** from `build.rs`.
- Verify generated files exist: `target/debug/build/normalizer-service-*/out/registry.rs` and `program_*.rs` (e.g., `program_whirlpool.rs`).
- (Optional) Fixture test: `cargo run -p normalizer-service -- fixture --path path/to/fixture.json` → prints normalized JSON.

---

## 4) Current Contracts (UI‑facing)

``

```json
{
  "ts": 1755214291,
  "dex": "orca",
  "pair": "SOL/USDC",
  "price": 1.7586,
  "src_sig": "...",
  "window_secs": 5
}
```

``

```json
{
  "ts": 1755211128,
  "slot": 359927220,
  "sig": "...",
  "dex": "orca",
  "token_in": "...",     // may be null (heuristic)
  "amount_in": 0.1234,     // may be null (heuristic)
  "token_out": "...",
  "amount_out": 5.6789,
  "signers": ["..."],
  "mints_in_tx": ["..."]
}
```

---

## 5) Next 2–3 Steps (2025‑09‑30)

1. **Restore swap extraction coverage**
   - Update the hotpath geyser subscription (and/or normalizer ingestion) so `SubscribeUpdateTransaction.meta` carries pre/post token balances; without them `normalizer_swaps_extracted` stays at zero and no swap events reach Kafka.
   - Once balances flow, ensure the generated extractors surface Raydium CLMM/AMM, Meteora DLMM, Phoenix, etc. into the venue-specific Kafka topics (`orca-swaps.parsed.v2`, `raydium-swaps.parsed`, `meteora-swaps.parsed`, `phoenix-swaps.parsed`).

2. **Restart & verify discovery once swaps flow**
   - After swap topics carry live data, confirm `discovery-service` metrics tick up (`http://127.0.0.1:9102/metrics`) and fix any schema drift.

3. **Refresh Grafana / UI observability**
   - Add charts for `multi_geyser_stream_messages_per_minute`, `dex_kafka_*`, and the normalizer swap counters.
   - Surface arbitrage counts (`alerts.arbitrage.opportunities`) alongside the parsed swap feeds for live decision-making.

---

## 6) Known Quirks & Fixes

- **Block time = 0** – Geyser snapshots don’t include block timestamps; enrich later via block stream or tolerate zero for now (discovery should handle missing values).
- **Partial extractor coverage** – Only Whirlpool swaps emit today; other venues still rely on the generated decoder stubs returning `None`.
- **Metrics wiring** – `normalizer_swaps_extracted` counter exists but may not reflect venue hits until we increment it inside the extractor loop.

---

## 7) Minimal Env Reference

Root `` (read by `run-dev.sh`):

```
KAFKA_BROKER=localhost:29092
ENDPOINT=<geyser-grpc-url>
X_TOKEN=<your-token>
ENABLE_DEX_KAFKA=true
DEX_TRANSACTIONS_TOPIC=dex-transactions
EVENT_BUS_CONFIG='{"enable_kafka":true,"kafka_topic":"alerts.arbitrage.opportunities", ... }'
```

Script echoes **ENDPOINT** / **X_TOKEN** at start so you can verify the right Geyser credentials.

---

## 8) Quick Start Checklist

1. Verify Kafka (`kafka-topics --list`) and ensure `dex-transactions`, `alerts.arbitrage.opportunities`, `orca-swaps.parsed.v2` exist.
2. Launch hotpath + normalizer (`systemctl status hotpath-arb-engine.service`, `normalizer-service.service`); watch logs for `DexKafkaPublisher` and “→ normalized” messages.
3. Confirm metrics:
   - `http://127.0.0.1:8088/metrics` → `dex_kafka_*`, `multi_geyser_*`
   - `http://127.0.0.1:9901/metrics` → `normalizer_transactions_processed`
4. If swaps are missing, start with Whirlpool (already emitting) then implement remaining venue extractors.

---

*End v1.2 — keep this at repo root under **`docs/ai-handoff.md`** as well if desired.*
