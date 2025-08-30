### MEVana Arbitrage Strategy & Architecture (AI-friendly)

Last updated: 2025-08-21 05:22 UTC

#### Goal
Ultra-low-latency arbitrage detection and execution across many venues (10+), with live, slot-coherent state, and a single-process hot path that operates entirely in memory using integers (fixed point).

#### High-level flow

```mermaid
flowchart LR
  G["Geyser/Yellowstone\n(account updates)"] --> D1["Venue decoders\n(Orca CLMM, Raydium CPMM, Meteora DLMM, Phoenix CLOB)"]
  D1 -->|protocol-native deltas| A["Edge Assembler\n(per-venue → normalized edges)"]
  A -->|edge.delta (compacted Kafka)| S["Searcher (HOT PATH)\n(in-process quoters + detector)"]
  S -->|execution request| X["Executor\n(Jito bundles)"]
  A -->|edge.snapshot| S
  D1 -->|telemetry| R[(Redis/Postgres/Grafana)]
  S -->|obs| R
  X -->|obs| R
```

#### Hot path rules (non-negotiable)
- No cross-process calls in the search loop.
- Integers only for quote-critical math (u128/i128 Q64.64). Floats only for logs.
- Precompute instruction metas per route/venue; hot path fills amounts + blockhash only.
- Single-threaded or pinned-thread detection/sizing loop.
- Freshness/coherency enforced by (slot, write_version) and age thresholds.

#### Services and responsibilities
- Venue decoders (one per program type):
  - Input: Geyser account updates
  - Output: protocol-native state deltas (Kafka), tagged with slot + write_version
  - Examples: orca.whirlpool.pool.delta, orca.whirlpool.tickarray.delta, raydium.cpmm.pool.delta, meteora.dlmm.binset.delta, phoenix.l2.delta

- Edge Assembler:
  - Subscribes to venue deltas, maintains minimal per-pool state
  - Emits normalized `edge.delta` messages (integers only) and a `edge.snapshot` for warm-start
  - Optional: per-pool compact windows (ticks/bins)

- Searcher (Hot Path, single process):
  - Subscribes to `edge.delta` (compacted topic or lock-free channel)
  - Keeps full edge graph in RAM
  - Every 1–2 ms or per-slot: runs detection + in-process quoting, sizes trades, produces execution requests

- Executor:
  - Fills amounts, sets CU/tip, simulates, submits Jito bundles, records outcomes

- Registry/Config:
  - Token allowlist, decimals, static accounts, risk flags

#### Normalized Edge model (integers only)
- Identity:
  - edge_id: u64 (hash of venue_id + pool_pubkey + direction)
  - venue_id: enum (OrcaWhirlpool, RaydiumCpmm, Phoenix, MeteoraDlmm, …)
  - pool_pubkey: [u8; 32]
  - token_in, token_out: [u8; 32]
  - direction: AtoB or BtoA

- Meta:
  - decimals_in, decimals_out: u8
  - fee_bps_total: u16 (LP + protocol)

- Quote-critical state (variant):
  - CLMM: sqrt_price_x64: u128, liquidity: u128, tick_current: i32, tick_spacing: i32, size_to_next_in: u128
  - CPMM: reserve_in: u128, reserve_out: u128
  - DLMM: active_bin: i32, bin_price_q64: u128, depth_hint_in: u128
  - CLOB: best_bid_px_q64: u128, best_bid_size: u64, best_ask_px_q64: u128, best_ask_size: u64

- Bookkeeping:
  - slot: u64, write_version: u64, updated_at_ns: u64

- Optional derived hints:
  - marginal_price_q64, size_to_next_boundary_in, cu_estimate

#### EdgeDelta message (Kafka) example (CLMM variant)
```
edge_id: u64
venue_id: u8
pool_pubkey: [u8;32]
token_in: [u8;32]
token_out: [u8;32]
direction: u8
fee_bps_total: u16
slot: u64
write_version: u64
payload_clmm: {
  sqrt_price_x64: u128,
  liquidity: u128,
  tick_current: i32,
  tick_spacing: i32,
  size_to_next_in: u128
}
```

#### Slot coherency strategy
- Per account: apply only if (slot, write_version) newer; drop stale/out-of-order
- In searcher: require legs fresh by both Δslot ≤ 1–2 and age ≤ X ms; or run strictly on slot boundaries using latest state per edge
- CLOBs (Phoenix): use event-queue sequence to ensure coherent L2 view

#### Candidate scaling & pruning
- Maintain adjacency lists keyed by `token_in`
- Keep top-K edges per token pair by marginal rate and capacity (e.g., K=5–10)
- Prune edges older than Y ms, with capacity below threshold, or untrusted tokens

#### CLMM specifics (Orca Whirlpools)
- Decoder correctness:
  - TickArray layout is `[Tick; 88]`, order: `start_tick_index`, `whirlpool`, `ticks`
  - Derive only 3 PDAs per pool: lower/current/upper (span = `tick_spacing * 88`)
  - Validate: discriminator_ok, whirlpool_ok, start_tick alignment, count initialized ticks

- In-process CLMM quoter (Q64.64, integers):
  - `quote_out(dx)`: walk ticks, apply fees, return dy and next state
  - Unit tests vs Orca SDK

- State to retain per pool (minimal):
  - sqrt_price_x64, liquidity, tick_current, tick_spacing, compact tick windows near price
  - fee_bps_total

#### No-RPC design and bootstrapping
- Hot path: absolutely no RPC/HTTP or cross-process calls
- Venue decoders: rely solely on Geyser updates
- Initial state (warm start) without RPC:
  - Maintain a compacted Kafka topic (or Redis) fed by decoders/assembler with last-known state; searcher consumes this `edge.snapshot` at startup
  - If no snapshot is available, the system will converge as Geyser updates arrive; initial edges may be incomplete until first writes
- Printing/demo: avoid JSON-logging full ticks (u128 overflow in JSON); log concise summaries only

#### Observability (minimum)
- Input lag per venue (ms) from Geyser to assembler/searcher
- Edge staleness distribution in the search loop
- Decode failures and discriminator mismatches
- Duplicate/stale drops by (slot, write_version)
- EV before/after tip+CU; landed vs failed

#### Step-by-step from current state
1) Finalize CLMM TickArray correctness and concise summaries (done; wiring in progress)
2) Implement integer CLMM quoter (Q64.64)
3) Define Edge/EdgeDelta (integers) and minimal producer/consumer
4) Build Edge Assembler for CLMM and emit edge.delta + edge.snapshot
5) Build Searcher (single process): ingest edges, run detection/sizing every 1–2 ms; produce execution requests
6) Build Executor: metas, CU, tip, simulate, submit bundle, record
7) Add second venue (CPMM or CLOB) with the same Edge model
8) Add pruning, sharding, and dynamic subscription for high-liquidity pools

#### Key takeaways
- Keep search/quote/execute tight and local (single process)
- Normalize on quoteable edges, not raw transactions
- Kafka/Redis for fan-out, replay, and warm start – not for interactive quoting
- Enforce simple freshness/coherency rules so legs are comparable
- Add venues by implementing a `Quoter` + tx template and plugging into the same Edge model


