# MEVana — AI Handoff Pack (v1.1)
Last updated: 2025-09-15

> Single doc for any assistant (or future you) to resume work fast. Includes current state, how to run, how to verify, and what’s next.

---

## 0) Snapshot — What’s Running / Topics

**Services (Rust):**

- `dex-stream-orca` → produces `` (Yellowstone Geyser → Kafka)
- `normalizer-orca` → consumes `orca-swaps`, emits ``, DLQ to ``
- `price-aggregator` → consumes `orca-swaps.parsed`, emits compacted ``
- `ui-gateway` → WS fan‑out for `prices.latest` and `orca-swaps.parsed`
- `ui-web` → React/Tailwind dashboard (Neon Carbon theme)

**Kafka Topics:**

- `orca-swaps` (protobuf, input)
- `orca-swaps.parsed` (JSON, normalized swaps)
- `orca-swaps.dlq` (JSON, errors)
- `prices.latest` (JSON, compacted)
- (optional) `orca-swaps.heartbeat`

**Note (Kafka warning):** "Due to limitations in metric names, topics with a period (`.`) or underscore (`_`) could collide…" — benign for dev. Prefer not mixing both in new topic names.

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

## 5) Next 2–3 Steps

1. **Wire codegen normalizer-service** into the loop

   - Extend `normalizer-orca` to call the generated decoders (or replace with `normalizer-service` binary per‑DEX).
   - Add output `...dex` topic for richer swap records.

2. **Raydium**

   - Add `profiles/raydium.yml` and IDLs (AMM + CLMM), run codegen (already compiling), point a Raydium stream → same pipeline.

3. **UI quality passes**

   - Focus drawer chart per pair (Lightweight‑Charts)
   - Virtualized swaps; filters (pair/DEX) with URL state
   - Coalesced WS frames (250ms) in `ui-gateway`

---

## 6) Known Quirks & Fixes

- **Kafka metric name warning** about `.` vs `_`: harmless in dev; prefer consistent naming for new topics.
- **Occasional null amounts** in `orca-swaps.parsed`: expected until codegen extractor is wired; DLQ captures broken frames.
- **ESLint v9 config**: use flat `eslint.config.js` (already set up) to avoid deprecated options warnings.

---

## 7) Minimal Env Reference

Root `` (read by `run-dev.sh`):

```
KAFKA_BROKER=localhost:29092
ENDPOINT=<geyser-grpc-url>
X_TOKEN=<your-token>
PRICES_TOPIC=prices.latest
SWAPS_TOPIC=orca-swaps.parsed
ORCA_IN=orca-swaps
ORCA_OUT=orca-swaps.parsed
ORCA_DLQ=orca-swaps.dlq
```

Script echoes **ENDPOINT** and **X\_TOKEN** on startup so you can confirm the right stream credentials.

---

## 8) If Starting Fresh (AI quick start)

1. Ensure Kafka running; create `prices.latest` compacted.
2. Run `./infra/scripts/run-dev.sh` (reads `.env.dev`).
3. Check `infra/logs/*.log` for errors; confirm WS green in UI.
4. If no data, verify Geyser creds (ENDPOINT, X\_TOKEN). If still empty, seed mock data into gateway or UI store to validate rendering.

---

*End v1.1 — keep this at repo root under **`docs/ai-handoff.md`** as well if desired.*

