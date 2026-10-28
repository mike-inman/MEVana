# Rust crates inventory and capability map — 2026-10-28 08:32 CDT

**Purpose:** record what already exists in `crates/`, what actually works, and what the research plan can reuse. The plan covers capture, then decoding, then exact replay, then ML ranking against baselines, then honest outcomes.

**Method:** six parallel read-only reviews read every Rust file in the 26 workspace crates, `crates/_archive`, and the deleted `multi-geyser-streams` source (recovered with `git show 14874a0d^`). About 100,000 lines were covered. Nothing was built, run or edited. The highest-impact claims were then checked by hand, as listed under "Verified directly". Everything else comes from code reading and carries file references for re-checking. The last full workspace build and test pass was on 2026-10-16 (`a3f604ce`); a passing build does not establish the correctness of anything below.

## 1. Bottom line

1. **No crate contains exact quote math.** Every price in the arbitrage path is an `f64` marginal price. The Whirlpool "quoter" uses approximate `f64` tick math with broken Q64 scaling and never crosses ticks.
2. **The HotPath engine's opportunity "detection" is fabricated.** It flags any transaction that touches a known DEX. Its sub-detectors fire on, for example, `signature.ends_with("c")` or `time % 100 < 5`. Profit is a constant plus an amount based on the number of accounts in the transaction.
3. **Recorded outcomes are estimates relabelled as results.** HotPath records success right after enqueueing. Executor "actual" profit is a simulated or estimated value. Reconciliation is a wallet balance difference with no slot.
4. **There is no real AI or ML anywhere.** The "AI optimization" uses wall-clock or constant features with random weights, and nothing consumes its output.
5. **Swap instruction decoding is mostly broken.** The code generator hashes camelCase instruction names, while Anchor programs hash snake_case names. Raydium AMM v4 and Phoenix are not Anchor programs at all, and the pool-state decoders have made-up offsets.
6. **A few pieces are genuinely good and reusable.** These are the Yellowstone client plumbing, `WhirlpoolMinimal` decoding and PDA helpers, `edge.proto` as a schema, Yellowstone token-balance parsing, Clerk JWT validation, the wallet-vault encryption module, and the Prometheus and audit-log helpers.

**What this means for the research question:** the existing system has never produced evidence about arbitrage profitability, positive or negative. That remains an open question. Answering it needs the new capture, exact quoting and deterministic replay described in sections 5 and 6.

## 2. Urgent security and safety findings

| # | Finding | Evidence | Status |
|---|---|---|---|
| S1 | **The public GitHub repository contains tracked credentials.** `crates/hotpath_arb_engine/.claude/settings.local.json` has been tracked since 2025-09-09. It holds about 10 Shyft-token-shaped UUIDs (`X_TOKEN=`), about 17 PostgreSQL URLs with passwords, and one `PGPASSWORD=`. Counts only; no values were printed. | `gh repo view` reports `PUBLIC`; `git ls-files` | **Verified.** The operator should rotate the Shyft token. Untracking the file is pending the operator's decision. |
| S2 | **The executor admin API fails open.** When `ADMIN_API_TOKEN`, `ADMIN_ALLOWED_SUBJECTS` and `CLERK_JWKS_URL` are all unset, every request is authorized (`AuthSource::Open`) on `0.0.0.0:9898`. `POST /admin/wallet/transfer` signs and sends real SOL from the keypair file. Dry-run mode does not cover it. | `executor-service/src/main.rs:241-262, 1161, 1550`; `wallet_service.rs:478-542` | **Verified.** Never run executor-service without admin auth configured. |
| S3 | **HotPath never checks `PAPER_TRADING`.** It is read only in the dead `LiveTradingEngine` (`main.rs:1399`). `flash_loan_enabled`, which defaults to true and can be set through an unauthenticated WebSocket or PUT on `0.0.0.0`, maps to `ExecutionMode::Live`. The bundles are fake and cannot land, so no funds move through this path, but the safety claim in `CLAUDE.md` is untrue. | `hotpath_arb_engine/src/main.rs:1399, 1968-1974, 2110-2115` | `PAPER_TRADING` read: **verified**. |
| S4 | **wallet-vault defaults are unsafe.** The master password defaults to a development string, authentication is off by default, the `X-User-ID` header is trusted, `GET /api/wallets/:id/keypair` returns the secret key, and `rotate` discards the old key. | `wallet-vault/src/main.rs:14-15`, `api.rs:133-135, 325-330`, `vault.rs:254-264` | From the review |
| S5 | **Credentials appear in logs and fallbacks.** api-service logs the full database URL, and kafka_db_writer, monitoring-service and token-registry carry hardcoded credentialed fallback URLs. | `api-service/src/bin/api-server.rs:22`; `kafka_db_writer:83, 343, 348`; `monitoring-service:124` | From the review |
| S6 | **api-service has a SQL injection.** The config-history query concatenates `parameter_name` and `applied_by` into the SQL string. | `api-service/src/database.rs:696, 701` | From the review |

## 3. Verified directly (2026-10-28)

- **The Anchor discriminator bug is real.** The generated `DISC_WHIRLPOOL_SWAPV2` is `[114,113,…]`, which is SHA-256 of `global:swapV2`. Anchor hashes `global:swap_v2`, which gives `[43,4,237,…]`. Whirlpool `swapV2` therefore never matches. Source: `normalizer-codegen/src/lib.rs:77, 274`; the built `generated_registry.rs:287`.
- **The `whirlpool-core` TickArray layout is wrong.** `types.rs:7-22` has no `initialized: bool` field and places `whirlpool` before `ticks` (9,900 bytes). The repository's `idls/whirlpool_idl.json` defines `start`, then `ticks[88]` of 113 bytes each, then `whirlpool`, for 9,988 bytes in total. Handoff 09 was corrected accordingly.
- **The fabricated detectors exist** at `hotpath_arb_engine/src/main.rs:3421, 3460, 3535`.
- **Intent is recorded as an outcome.** `SMART_FILTER.record_result(&event, true, net_profit)` runs at enqueue time (`main.rs:2665-2671`).
- **S1 and S2** above.

## 4. Capability map against the research pipeline

```
 Geyser ──► [1 Capture] ──► [2 Decode state] ──► [3 Exact quote + replay] ──► [4 Rank vs baselines] ──► [5 Honest outcomes]
            NOT PRESENT      partial: Whirlpool    NOT PRESENT (f64 only;      NOT PRESENT (no ML;      contracts exist in C#
            (taps exist,     pool OK; TickArray    replay non-deterministic,   f64 2/3-leg screen is    (Stage 0); Rust records
            no recorder)     fixed in 09; others   panics, drops signals)      a weak baseline)         estimates as results
                             broken/stub
```

| Stage | What exists | Usable? | Gap |
|---|---|---|---|
| 1 Capture | Geyser clients: `executor-service/live_stream` (accounts; no reconnect), `hotpath main.rs:4034-4141` (transactions only; Processed; reconnect only in multi-endpoint mode), `geyser_probe`, the old `multi-geyser-streams`. HotPath republishes raw transactions to Kafka `dex-transactions`, dropping them when the queue is full. | Plumbing only | No lossless recorder. **Handoff 09** builds `geyser-capture`. |
| 2 Decode, Whirlpool | `whirlpool-core::decode_whirlpool` (`WhirlpoolMinimal` layout appears correct), `pda.rs` | Yes | TickArray layout (fixed in 09). No tests until 09. |
| 2 Decode, other DEXs | Raydium CLMM in `live_stream` (missing `bump` and padding) and in `raydium_map` (made-up offsets). AMM v4 is a stub, and there is no CPMM decoder. Meteora layouts are made up. The Phoenix price is a vault balance. | No | Rewrite from the official IDLs and program source, with golden tests. |
| 2 Decode, swap instructions | `normalizer-codegen` → `normalizer-generated`; `normalizer-runtime` | Partially | The discriminator case bug; non-Anchor programs; no address-lookup-table keys or inner (CPI) instructions; owner-blind amount inference; mislabeled Meteora IDLs and program IDs. |
| 3 Quote math | `whirlpool-core/math.rs` and `quoter.rs` (approximate `f64`, broken scaling, no `liquidity_net`); `arb-engine` (`f64` product of prices); `edge-assembler` (spot price only) | No | An exact integer Whirlpool swap port (or Orca's official core crate) with golden tests against on-chain swaps. |
| 3 Replay | `arb-engine/bin/arb_replay` | Shell only | It panics (`tokio::spawn` without a runtime), hangs at EOF, drops everything older than 250 ms by wall clock, is non-deterministic, and drops signals. It replays derived `EdgeDelta` frames, not raw captures. |
| 3 Graph and cycles | `arb-engine/edge_integration.rs` 2-leg and 3-leg search | Skeleton | Direction is swapped twice (C5), token alignment is not checked, fees are in mixed units (C2), and real Whirlpool edges always have zero size (C4). |
| 4 Ranking and ML | None (`ai_hooks`, `AIOptimizationEngine` and `ai_strategy_engine` are not ML) | No | Built in C#/Azure per the 10-06 plan. The `f64` screen can serve as a baseline only after it is fixed. |
| 5 Outcomes | C# Stage 0 contracts (dataset, request, result, ranking, outcome; handoffs 07 and 08). `executor-service` migrations 0001-0009 and `ExecutionStatus` | Contracts yes; Rust no | Rust outcome recording must be rebuilt on the C# contract. The executor's 0007 migration backfills reconciled profit from actual profit. |

## 5. Crate-by-crate inventory

Verdict key: **R** = reuse as is, **RF** = reuse with fixes, **Ref** = reference only, **D** = dead or misleading.

### Market data, decoding and registries

| Crate | Lines | What it actually is | Verdict |
|---|---|---|---|
| `whirlpool-core` | 708 | Whirlpool decoding, PDAs and an approximate quoter. No tests. | **RF** for decode, `pda` and `WhirlpoolMinimal`; fix TickArray. **Ref** for `math`, `quoter` and `tick_window` (rewrite). |
| `normalizer-runtime` | 792 | `NormalizedSwapV1` and Yellowstone context conversion. Prices are `f64`. Top-level instructions and static keys only. 3 tests. | **RF**: exact price; address-lookup-table and inner instructions; more layout types; golden tests. |
| `normalizer-codegen` | 859 | Generates decoders from profiles and IDL account order. Two divergent generators. | **RF**: snake_case hashing or the IDL discriminator, a tag-byte path, fail hard on missing instructions, delete one generator. |
| `normalizer-generated` | 1,526 | `include!`s the build output; `src/program_*.rs` are stale and unused. | **RF** through codegen. **D** for `program_*.rs`. |
| `normalizer-service` | 2,578 | Converts Kafka `dex-transactions` into swap JSON. Pool decoders always return `None` or use made-up offsets. `meta_ok` is forced true, `block_time` is the current time. Fixtures are unusable (`data_b64:"AQ=="`). 0 tests. | **RF** for the `tap_orca_extract` loop and the Redis decimals cache. **D** for the map decoders, fixtures, `arb_watch` and `tap_orca_decode`/`debug`. |
| `mevana-idl-registry` | 427 | IDL fetcher that is never called; not a workspace member. | **Ref** for `idls/` (verify each file; several are mislabeled). |
| `dex-registry` | 2,834 | Planned metadata service with made-up program IDs, stub plugins and placeholder tokens. | **D** |
| `token-registry-service` | 957 | Real mint decimals from RPC (byte 44), freshness merge. No owner check. | **RF**: add a token-program owner check; drop the deprecated GitHub token list. |
| `price-aggregator` | 228 | `f64` out/in ratio. Its input schema does not match `NormalizedSwapV1`, so it never prices anything. | **D** |
| `multi-geyser-streams` | 0 (729 in history) | Old three-stream transaction monitor. | **Ref** for the subscription and reconnect skeleton (used in 09). |

### Detection, edges and signals

| Crate | Lines | What it actually is | Verdict |
|---|---|---|---|
| `edge-assembler` | 1,638 | Kafka Whirlpool JSON to `EdgeDelta`. The tick window is always empty, so size is 0. Fee units are wrong. Pool state freezes at first sight. `venue_handlers` has a fake program ID and discriminator. | **R** for `edge.proto` as a schema (extend it). **Ref** for `main`/`pool_manager`. **D** for `venue_handlers` and `depth_builder`. |
| `arb-engine` | 9,581 | `f64` 2-leg and 3-leg cycle screen with an HTTP dashboard. The circuit breaker never gates processing. The Jupiter and executor integration are stubs. Synthetic modes can fabricate metrics. 21 tests, all synthetic. | **RF** for the `arb_replay` shell, `edge_verifier` and the cycle skeleton. **R** for `hotpath` (ring buffer), `stats` and `circuit` (metrics only). **D** for `ai_hooks` as ML, Jupiter, `fringe_risk_manager` and the uncompiled files. |
| `hotpath_arb_engine` | 20,335 | Geyser transaction tap with fabricated detection and simulated execution. Two `PropertyStore` singletons, so UI and database thresholds never reach the filter. Unbounded queues, a lock-order deadlock risk in `SmartFilter`, and a task leaked per trade. 13 tests, none for detection or execution. | **RF** for the Geyser builders (`main.rs:4034-4141`), the raw-tap idea, the `summarize_swaps` integer bps, `opportunity_from_signal` and the health endpoints. **Ref** for `smart_filter` as a baseline after fixes. **D** for the detectors, every AI module, Jito, templates, mock wallet signing and the `/wallet/*` and `/ai/*` routes. |
| `mevana-engine` | 358 | Plugin prototype that returns nothing; no crate depends on it. | **D** (the `DexPlugin` trait is at most a reference). |
| `shared-protocol` | 159 | `ArbSignalV1` serde types; `is_paper_trading()` returns false. | **RF**: add per-leg quotes, the slot and `write_version` used, and a policy ID. |

### Execution, flash loans and wallets

| Crate | Lines | What it actually is | Verdict |
|---|---|---|---|
| `executor-service` | 13,415 | Kafka to validate, quote, build a placeholder bundle, simulate and usually stop (dry-run). Bundle instructions are made up and signed by `Keypair::new()`. The Jito client is a non-standard REST call. "Actual" profit is an estimate. The Kafka offset is committed even on errors. Admin auth fails open (S2). | **RF** for `live_stream` (reconnect, layouts, raw bytes), `geyser_probe`, `replay_export` (column types), `clerk_auth`, `metrics`, `audit_log`/`hash_chain_logger` and the simulation token-delta parsing. **Ref** for the dry-run gate, `ExecutionStatus` and the reconciliation idea. **D** for `bundle_builder`, `execution_orchestrator`, `jito_optimizer`, `flash_loan_optimizer` and the wallet transfer and airdrop routes. |
| `flash-loan-providers` | 1,920 | Static quotes. Solend encoding is plausible; Marginfi is made up; Kamino and Port route to the stub receiver. 11 tests. | **Ref** (trait shape, Solend encoding). |
| `flash-loan-receiver` | 199 | On-chain stub that only logs. | **D** |
| `wallet-vault` | 1,880 | Real Argon2id/AES-GCM/ChaCha20 encryption with unsafe API defaults (S4). Not used by the executor. | **R** for the encryption module. **Ref** for the rest. |
| `wallet-metrics-exporter` | 704 | Wallet balance and USD Prometheus exporter; pulls in all of `executor-service`. | **D** for research. |

### APIs, monitoring and support

| Crate | Lines | What it actually is | Verdict |
|---|---|---|---|
| `api-service` | 4,384 | Axum REST with no authentication, plus a Kafka-to-WebSocket/SSE gateway. `ResilientDatabase::query` turns every bind into a string, so most parameterized queries fail. SQL injection (S6). 12 tests. | **RF** for the gateway (`fanout`, `routes`, `sse`; it is what MEVana-UI uses). **R** for `metrics.rs` and `request_id`. **Ref** for REST and DB (the Research API will use typed sqlx). **D** for `auth.rs` and `proxy.rs`. |
| `monitoring-service` | 779 | Reads Redis and DB aggregates. Its queue consumer can never receive anything, because the queue lives in another process. Latencies are hardcoded. Duplicates api-service. | **Ref** |
| `discovery-service` | 392 | Unknown-program tracker. Nothing produces its input and nothing consumes its "AI analysis" output. | **D** (the fingerprinting idea is at most a reference). |
| `kafka_db_writer` | 355 | Batches Kafka into the DB but never commits offsets, drops batches on error, and writes zero placeholder values. | **Ref** |
| `resilience-framework` | 1,821 | Circuit breaker, retry (constant delay, no jitter), dead-letter queue (no migration), degradation. Effectively unused in live code. 9 tests. | **RF** for the circuit breaker and DLQ. **RF** for retry (needs real backoff). **D** for `database.rs`. |
| `shared-core` | 1,489 | Program IDs: `JUPITER_V6` holds the JUP4 ID, and `RAYDIUM_CLMM` and `PUMP_FUN` need verifying. `Config` serializes `x_token`. `port_manager` silently reassigns ports and contains `kill -9`. 12 tests. | **RF** for `programs` (correct IDs; add a canonical-ID test) and `database_config`. **D** for `port_manager`. **Ref** for `port_config` and `wallet_safety`. |

### `crates/_archive` (not workspace members)

| Crate | Verdict |
|---|---|
| `_archive/api-service` | **D**: an older copy of the active api-service. |
| `_archive/api-service-v2-original` | **Ref**, and it does not compile. Possibly worth mining for the Research API: metric path normalisation (`middleware/metrics.rs:334`), security headers and CSP (`security.rs`), the rate limiter and the `audit_logs` DDL. |
| `_archive/system-orchestrator` | **Ref**: tiered startup with health gating (`startup_sequence.rs`, `health_checker.rs`). Do not revive it; it uses `pkill -f` and auto-restarts trading services. |

## 6. Cross-cutting defects

- **Program IDs:** wrong or fake IDs appear in `shared-core` (JUPITER_V6 is JUP4), hotpath (the Jupiter v6 subscription ID differs from the one the detector uses), `edge-assembler/venue_handlers`, `dex-registry`, `bundle_builder`, and the normalizer's DLMM/DAMM profiles. A single canonical, tested registry is needed.
- **Units:** Whirlpool `fee_rate` (units of 1e-6) is treated as basis points in `edge-assembler`, `executor live_stream` and `arb-engine`. Decimals are hardcoded to 9 or 9/6.
- **Floating point in money and price math:** found in nearly every crate. The C# contracts already require integer atoms.
- **Clocks:** wall-clock `updated_at_ns`, `block_time` = now, and wall-clock staleness make replay impossible.
- **Silent data loss:** `try_send` drops in the hotpath Kafka tap, the arb-engine signal queue and the live_stream snapshots. Failed batches are dropped in async_history_buffer and kafka_db_writer. Offsets are committed on error in the executor.
- **Port conflicts:**
  - 8085: token-registry, dex-registry, `tap_orca_extract` and arb-engine (which actually defaults to 8080).
  - 8090: wallet-vault and monitoring.
  - 8091: api-service and monitoring.
  - 9898: token-registry and executor admin.
  - Redis is listed as 6379 in the code and `ports.toml` but as 6380 in `CLAUDE.md`.
- **Duplicated services:** api-service, monitoring-service and kafka_db_writer all read or write `arbitrage_opportunities`. hotpath and executor each have their own Geyser stacks, Jito code and AI code.
- **Two Solana SDK lines** (2.1.x and 3.x in arb-engine) are compiled into the workspace.

## 7. Recommended sequence (no change to trading behaviour)

1. **Now (operator):** rotate the Shyft token exposed by S1, then untrack and ignore the settings file. Do not run executor-service without admin auth (S2).
2. **Handoff 09:** read-only capture, plus the `whirlpool-core` TickArray fix and its first tests. Already written and corrected for this inventory.
3. **Handoff 10, exact Whirlpool quoting:** an integer port of Orca's swap math with tick crossing and `liquidity_net`, golden-tested against captured on-chain swaps and their pre and post state. This is what makes replay results mean anything.
4. **Handoff 11, deterministic replay:** a new replay core over raw captures, with an event-time clock, slot and `write_version` ordering, lossless output, and `Simulated` `ExperimentResult` output. Reuse the `arb_replay` shell only if that is simpler. Include a corrected 2-leg and 3-leg baseline (direction, token alignment, fee units).
5. **Handoffs 12 and 13:** the Research API (typed sqlx; optionally the archived v2 middleware patterns) and the MEVana-UI research page.
6. **Hygiene backlog:**
   - a canonical program-ID registry with tests;
   - fix the discriminator case in codegen;
   - make admin and wallet-vault authentication fail closed;
   - stop logging database URLs;
   - fix the SQL injection;
   - remove or quarantine the fabricated detectors and "AI" modules, so that dashboards stop showing fake opportunities.
