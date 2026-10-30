# 2026-10-30 08:59 CDT — Handoff 12: Tick-array snapshots and slot-correct replay

## Purpose

Handoff 11 produced honest results that tell us very little, because most candidates could not be evaluated:

| Pair | `state_gap` | `insufficient_state` | Evaluable |
| --- | ---: | ---: | ---: |
| A SOL/USDC | 26.75% | 5.20% | 68.05% |
| B SOL/USDT | 56.16% | 29.25% | 14.58% |
| C USDC/USDT | 58.01% | 29.73% | 12.25% |

The operator's verification on 2026-10-30 found two causes, and this slice fixes both:

1. **The replay mis-reads slot semantics.** It produces most of the `state_gap` share, and it is fixed offline.
2. **The capture has no initial tick-array state.** It produces most of the `insufficient_state` share, and it is fixed with a small, bounded RPC snapshot taken at capture time.

Then re-run the same deterministic analysis twice:
- on the handoff 09 capture, so the effect of the slot fix shows on its own;
- on one new ≤10-minute capture with snapshots.

This slice does **not** trade, sign, train, add triangles, change the request contract, or use a server host.

**Numbering:** the Research API and the MEVana-UI page, previously "handoffs 12–13", become handoffs 13–14.

## Findings (operator-verified, 2026-10-30)

I decoded the slot-update frames (`SubscribeUpdateSlot`: `slot`, `parent`, `status`) from `~/MEVanaData/captures/handoff09-20261029/capture-1793330490539629000/`:
- **2,653 slots** were seen, and every one carries a `parent`. Every parent was itself seen.
- **8 `slot_discontinuity` gaps were recorded** by `geyser-capture/src/source.rs:320-330`, which flags any jump of `slot > prev + 1`:
  - **7 are skipped leader slots.** The new slot's `parent` equals the previous slot, and the slot later reached Confirmed and Finalized. No data was lost.
  - **1 is a fork.** Slots 463132684–686 were only ever Processed, never Confirmed. Slot 463132688's parent is 463132683, so 684–686 were an abandoned fork.
- **The replay mishandles both cases** (`research-replay/src/lib.rs` `MarketState::gap`, `apply`):
  - Every discontinuity marks **all** pools stale until each pool's own account is rewritten. Thin pools are rarely rewritten, so they stay stale for long stretches. This is the bulk of `state_gap`.
  - Writes from the abandoned fork slots were applied as if they were canonical state.
- **Tick arrays enter state only when they change during capture.** An array that exists but never changes is never seen, so quotes that cross it are `insufficient_state`. Arrays that don't exist on-chain are indistinguishable from arrays we never saw.

**RPC preflight (operator, 2026-10-30).** Tatum mainnet RPC at `https://solana-mainnet.gateway.tatum.io/`, header `x-api-key`, key `TATUM_MAINNET`:
- `getSlot` (confirmed) returned in 0.6 s;
- `getMultipleAccounts` (confirmed, base64) returned the USDC/USDT pool at 653 bytes in 0.3 s.

`SODAE.IO_RPC_URL` is plaintext `http://…:8899` and **must not be used**. The same applies to Sodae's plaintext Geyser URL.

## Execution model

Implement this directly in this repository with an interactive coding-assistant session (Codex or Claude Code), following the OpenSpec workflow.
- Do **not** use the AgentEngineering factory.
- Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/snapshot-20261030`, on branch `feat/capture-snapshot-20261030`, from `origin/main` (`737cf1e0` or later).
- Merging is the operator's decision.
- **Toolchain:** use `~/.cargo/bin/cargo`, and `cargo -V` must show `1.92.0-nightly`. Homebrew results are invalid (see `CLAUDE.md`).
- **Order:** Part A (offline) must be finished and validated **before** any network use in Part B.

## Part A — Slot-correct replay (offline, no network)

Make these changes in `crates/research-replay`.

**A1. Slot graph.**
- Build it from connection-0 slot frames: per slot, the `parent` plus the first receive time of each status (Processed, Confirmed, Finalized, and Dead if present).
- Use the `recv_ns` and capture sequence already in the frames. Do not use wall-clock time.

**A2. State commitment: `confirmed` by default.**
- A slot's account writes are buffered, and applied in (slot, write_version, sequence) order **at the event-time of that slot's first Confirmed status**.
- Writes in slots that never reach Confirmed in the capture are **dropped**. Count them as `droppedForkWrites`, with their slots, in the summary and manifest.
- Evaluation boundaries are confirmed slots, in confirmation order. L is counted in confirmed slots.
- Keep a `processed` mode, which applies writes as they arrive (handoff 11 behavior, minus the false gaps), as a sensitivity run only.

**A3. Gap semantics.**
- A slot jump is **not** a gap when the new slot's `parent` was seen. That covers skipped leader slots.
- A **true gap** is any of:
  - a capture gap frame whose reason is not `slot_discontinuity` (`ping_disconnect`, reconnect, stall);
  - a confirmed slot whose `parent` was never seen;
  - a `slot_discontinuity` frame that the parent chain does not explain.
- Only true gaps mark pools stale.
- Staleness clears on a covering post-gap snapshot (Part B), if one exists. Otherwise it clears on the pool's next write, as before. The closeout says which rule applied.
- Record every gap with its classification in the manifest.

**A4. Determinism and versions.**
- Bump the engine version to `research-replay-0_2_0`.
- Two identical runs must stay byte-identical.
- Handoff 11's committed fixtures and results stay **unchanged** as history. New results get new `resultId`s and fixture files.

**A5. Rerun on the handoff 09 session.** Use the same parameters as handoff 11, defaults only:
- produce A, B and C in `confirmed` mode;
- produce A in `processed` mode as a sensitivity run;
- report a per-pair table comparing the bucket shares against handoff 11, plus `droppedForkWrites` and the gap classification.

Expected outcomes:
- 7 skipped slots and 1 fork. **0 true gaps.**
- `state_gap` close to 0%.
- Writes in slots 463132684–686 dropped.

If these don't hold, stop and report why.

**A6. Tests.**
- Use synthetic slot graphs covering a skipped slot, a fork, an orphan parent and a reconnect gap frame.
- Check that fork writes never reach state.
- Check that a skipped slot never marks stale.

## Part B — Capture snapshots (bounded RPC)

Make these changes in `crates/geyser-capture`. Then capture one new session.

**B1. Snapshot source.** Add an opt-in flag, `--snapshot-rpc tatum`. With it off, behavior and output are byte-for-byte unchanged.
- The endpoint is fixed at `https://solana-mainnet.gateway.tatum.io/`. Refuse anything that is not `https`.
- Use header `x-api-key`, with the key `TATUM_MAINNET` read from `.env` under the existing credential rules:
  - read only that key;
  - never print, log, copy or commit it;
  - never send it to any other host;
  - never send a Sodae key to Tatum.

**B2. Snapshot procedure.** The Geyser stream (Sodae TLS, as in handoff 09) starts **first**. After its first Confirmed slot:
1. Call `getMultipleAccounts` for all allowlisted pools, with `commitment: confirmed` and `encoding: base64`. Record context slot `C_pool`.
2. For each returned pool:
   - derive the tick-array PDAs for start offsets **−6..+6 array spans** around its current tick, using `whirlpool_core::exact::derive_tick_array_address`, which takes the decimal seed;
   - that gives 13 per pool, matching the quoter's 6-array reach in each direction.
3. Fetch those arrays in batches of ≤100 keys, with `commitment: confirmed` and `minContextSlot: C_pool`. Record each batch's context slot.
4. For every requested key, record:
   - the key and the batch context slot;
   - `present` or `absent`;
   - for present accounts: the owner, lamports, the raw data, and its SHA-256.

   Reject present accounts whose owner is not the Whirlpool program.

**B3. When to snapshot.**
- Once at start.
- Again after each **true** reconnect gap on the accounts connection.
- **Never periodically.**

**B4. Budgets (hard).**
- ≤ 6 RPC calls per snapshot;
- ≤ 5 snapshots per session;
- ≤ 25 MB of RPC response bytes per session;
- 10 s timeout per call;
- at most 2 retries with backoff, and no retry on 401/403.

When a snapshot fails:
- write a `snapshot_failed` record (no secrets) and continue streaming;
- the replay then treats the affected pools under the no-snapshot rules.

**B5. Format.**
- Add frame kind **2 = snapshot**. Its payload is a versioned record, either JSON with base64 data or a compact binary layout; document whichever you choose.
- Bump the session/manifest `formatVersion` to 2.
- File manifests count snapshot frames, accounts, absent accounts, calls and RPC bytes.
- `capture_inspect` validates kind 2. `research-replay` accepts formatVersion 1 and 2, and rejects unknown kinds.
- Old captures must still inspect and replay unchanged.

**B6. Replay use** (in `research-replay`).
- A snapshot account at context slot C seeds state as of C.
- Stream writes for that account at slot ≤ C are ignored. Writes at slot > C apply under A2.
- An array that the snapshot proved **absent** is never treated as missing data. If a quote's `MissingTickArray { start }` points at an array known to be absent (and not created later), the candidate gets the new reason **`tick_array_absent`**, meaning not executable on-chain. Otherwise it stays `insufficient_state`.
- `reasonCode` is a free identifier in the v1 result contract (`ExperimentResultSyntaxValidator.cs:96`), so no contract change is needed. Confirm this with a test.

**B7. Capture run.** Exactly one session:
- `--duration 600` or less, Processed commitment;
- the same 18-pool allowlist and providers as handoff 09, plus `--snapshot-rpc tatum`;
- output under `~/MEVanaData/captures/handoff12-20261030/`.

Before and after the run:
- check for leftover capture processes;
- check for provider sockets (a Codex browser keep-alive at about 3 B/s doesn't count).

Report:
- Sodae bytes, against the trial's 5 GB / 7-day budget from 2026-10-29 (about 130 MB used so far);
- Tatum calls and bytes.

Run no other live sessions. If the preflight or snapshot fails twice, stop and report.

**B8. Replay of the new session.**
- Defaults for A, B and C in `confirmed` mode, with two runs that must be byte-identical.
- For A only: L = 2, the cost run, and the `processed`-mode sensitivity.
- C# validation of each new result with the existing, unchanged v1 validators, via new test fixtures and catalog entries.

## Constraints

- **Network:** only Part B's single capture session (Sodae Geyser over TLS) plus the bounded Tatum snapshot calls. Nothing else, and nothing left running.
- **No signing,** no wallets, no transactions. Trading stays disabled.
- **Integer-only quote math** via `whirlpool_core::exact`. Do not change `exact.rs`. Do not read post-2025-02-27 Orca material.
- **Rust changes** are limited to `geyser-capture`, `research-replay` and `Cargo.lock` additions. Any new HTTP client must use rustls with default features off, with no OpenSSL. Prefer `reqwest` if it is already locked.
- **.NET changes:** test fixtures and catalog entries only. Existing contract behavior must not change.
- **Not changed:** `hotpath_arb_engine`, `executor-service`, `shared-core`, `arb-engine`, `edge-assembler`; UI, Docker, Kafka and PostgreSQL.
- Date every document. Commit and push all work; lose nothing.

## Validation (pinned toolchain)

- `cargo fmt` check, offline locked `clippy -D warnings`, and tests for `geyser-capture`, `research-replay` and `whirlpool-core`;
- `dotnet test dotnet/MEVana.slnx`;
- `openspec validate --all --strict`;
- `./scripts/ci/repo_hygiene.sh`;
- `git diff --check`;
- a non-printing credential scan over the diff **and** over the new capture's manifests and logs.

## Closeout

Write a dated closeout doc containing:
- the Part A comparison against handoff 11 on the old capture;
- Part B's capture stats: Sodae bytes, Tatum calls and bytes, snapshot coverage, and present/absent counts;
- new per-pair policy tables and pair A sensitivities;
- determinism hashes;
- limitations: dev/test feed, 10 minutes, laptop host, no fill or inclusion modeling, heuristic ranking.

A zero-opportunity result is still a valid outcome. Archive the OpenSpec change. Push the branch clean and synchronized.

## Exit criteria

The slice either stopped with a precise report, **or** all of the following are true:
- slot-correct replay is specified, tested and validated against the handoff 09 session;
- one snapshot-backed capture exists within budget;
- new results for the three pairs validate under unchanged v1 validators;
- runs are deterministic;
- everything above is documented, archived and pushed.

## Later slices

- Longer captures on a server host (operator decision on Vultr or Hetzner and a provider plan).
- Request contract v2 and triangles.
- First learned ranker.
- Adaptive-fee quoting.
- Workspace `rdkafka-sys` build remedy.
- Handoffs 13–14: the Research API and MEVana-UI research page.
