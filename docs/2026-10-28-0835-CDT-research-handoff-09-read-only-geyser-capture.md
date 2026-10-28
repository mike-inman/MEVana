# 2026-10-28 08:35 CDT — Handoff 09: Read-only Geyser market-data capture

## Purpose

Start recording real Solana market data so that MEVana can eventually answer the research question: does ranked route selection beat simple baselines on executable, cost-adjusted opportunities? Contracts cannot answer that, and evidence accrues only in calendar time, so capture starts now. It runs alongside the later API and UI slices.

This slice builds a **record-only** capture tool for Orca Whirlpool state, checks that its files are intact, and exercises the existing Whirlpool decoders against real bytes. It **does not** detect opportunities, simulate trades, replay data, or trade.

## Entry state and prerequisite

- **Do not start until handoff 08 is closed out.** That means `docs/…-handoff-08-closeout.md` is committed, five specs pass strict validation, and local and GitHub `main` are in sync. If 08 is incomplete, stop and report that instead.
- The Rust toolchain is pinned to `nightly-2025-09-15` (`rust-toolchain.toml`). `yellowstone-grpc-client` and `yellowstone-grpc-proto` 4.2.1 are already in `Cargo.lock`.
- These are the reusable references; read them but do not modify them:
  - **Account subscription template:** `crates/executor-service/src/live_stream/mod.rs:365-418`.
    - Builder: `build_from_shared(..).x_token(..).connect_timeout(10s).timeout(10s).max_decoding_message_size(..)`.
    - `SubscribeRequestFilterAccounts` filters by owner, at Processed commitment.
    - This template has **no reconnect loop** (`:357`); do not copy that gap.
  - **TLS setup:** `crates/hotpath_arb_engine/src/main.rs:4034-4055` (`tls_config(with_native_roots())`).
  - **Minimal probe:** `crates/executor-service/src/bin/geyser_probe.rs:42-122`.
  - **Whirlpool program ID:** `ORCA_WHIRLPOOL` in `crates/shared-core/src/lib.rs:88`.
  - **Whirlpool decoders:** `crates/whirlpool-core/src/decode.rs:25,33` (`decode_whirlpool` and `decode_tick_array`, which check the Anchor discriminator and then Borsh-decode). Types are in `src/types.rs`. **These decoders currently have no tests.**
- Credentials:
  - The endpoint comes from `GEYSER_ENDPOINT`, falling back to `ENDPOINT`.
  - The token comes from `GEYSER_X_TOKEN`, falling back to `X_TOKEN` (and `SHYFT_GEYSER_X_TOKEN`, as in `executor-service/src/config.rs:94`).
  - The operator supplies these through the environment. Do not open, print, copy, edit or create `.env` files or other secret files.
- **The OpenSpec project context blocks this slice and must be narrowed.** Line 14 of `openspec/config.yaml` forbids live-data and provider calls and Rust changes. That restriction was written for the contract slices, but as project-wide context it would apply here.
- Raydium is out of scope. Its CLMM decoder is private and untested, AMM v4 decoding is a stub, and there is no CPMM account decoder.

## Work

1. **Update OpenSpec and propose the change.**
   - Edit `openspec/config.yaml` so the "no live-data/provider calls, Rust…" restriction applies to **research contract changes** only. Add standing capture rules:
     - capture is read-only market-data recording;
     - no signing, wallets, transaction construction or submission;
     - tokens never appear in logs, files, manifests, errors or commits;
     - captured market data is never committed, apart from the small fixtures described in step 6.
   - Then create the OpenSpec change `add-geyser-market-data-capture` (capability `market-data-capture`) with a proposal, design, tasks and spec delta covering the requirements in steps 3 to 5. Validate it strictly.
2. **Run a connectivity preflight before writing the capture tool.** Using the existing `geyser_probe` approach or a 60-second throwaway subscription, open one read-only account subscription filtered to the Whirlpool owner. Record:
   - connect time;
   - time to the first update;
   - update count;
   - any error class. Redact errors, and do not print the endpoint's query string or the token.

   **If authentication or connection fails, stop.** Write the closeout with the precise blocker and do not continue.
3. **Create a new crate, `crates/geyser-capture`.** Add it to the workspace `members` but **not** to `default-members`. Depend only on crates already in `Cargo.lock` at their locked versions. After building, confirm that the lockfile diff only adds the new crate and introduces no version changes. Only use a compression crate if it is already locked; otherwise write uncompressed files and report the volume.
   - **Subscription:** accounts owned by `ORCA_WHIRLPOOL`, split into two named filters, plus slot updates.
     - (a) **Pool accounts:** all Whirlpool pool accounts, selected by data size.
     - (b) **TickArray accounts:** only those whose stored `whirlpool` field matches the pool allowlist, using memcmp filters.
     - Derive each data size and memcmp offset from `whirlpool-core` types and discriminators, and prove them with unit tests. Do not hard-code values copied from memory or the web.
     - Exclude position accounts.
     - **Commitment:** Processed by default and configurable. Record it in the manifests.
     - Slot updates let replay reconcile processed state with confirmed and finalized status later.
   - **Optional transaction stream.** Behind a flag that defaults to on, subscribe to non-vote transactions, **including failed ones**, whose `account_include` names an allowlisted pool. A failed transaction is evidence of a contested opportunity.
   - **Keepalive and reconnect.**
     - Answer Yellowstone pings as the client library expects.
     - Reconnect with capped exponential backoff and jitter, at most 30 seconds.
     - Record a `stall` event when no message arrives within a configurable timeout.
     - Every disconnect, reconnect, stall and slot discontinuity is written as a **gap record**. Gaps are never silently absorbed.
   - **Capture format v1.** Document it in the crate README and the spec.
     - Files are append-only and made of length-delimited frames.
     - Each frame holds a monotonically increasing sequence number, the local receive time (UTC nanoseconds) and the **unmodified encoded `SubscribeUpdate` bytes**.
     - Do not decode or filter frames on the way into the file.
     - Write to `*.part`, `fsync`, then atomically rename on rotation, which happens by size or once per hour.
     - A crash leaves a `.part` file, which the inspector reports as incomplete and never catalogs.
   - **Manifests.**
     - **Per file:**
       - format version and SHA-256;
       - byte and frame counts;
       - first and last slot, and first and last receive time in UTC;
       - commitment and filters, including the allowlist;
       - endpoint **host only**;
       - crate version and git commit;
       - counts and details of the gap records.
     - **Per session:** a manifest that lists the per-file manifests.
     - Map the manifest fields to the C# `DatasetMetadata` contract (`Id`, `MetadataVersion`, `Provenance`, coverage UTC) and document the mapping. Note that `Provenance` rejects `/`, `\` and `://`, so the endpoint stays in the capture manifest, not in the dataset provenance.
   - **Configuration.** Use command-line options or environment variables for the output directory, rotation, commitment, allowlist file, transaction flag and stall timeout.
     - Default the output to `~/MEVanaData/captures/`, which is outside the repository and not `/tmp`. Add the relevant ignore patterns anyway.
     - On startup, log the configuration with secrets redacted.
4. **Write a `capture_inspect` binary in the same crate.** It must:
   - check frame integrity and manifest hashes, and report `.part` files and gaps;
   - decode captured pool and TickArray accounts with `whirlpool-core`, reporting decode success and failure counts by account type;
   - produce a **pool inventory**: pool address, mint A and B, tick spacing and fee rate (as decoded), and update count.
5. **Tests** (no network): frame round trip, rotation and atomic rename, `.part` handling, manifest hash correctness, gap recording, backoff bounds, filter offsets and sizes, and secret redaction. For gap recording and backoff, use a mock update source, and introduce a source trait if needed.
6. **Decoder fixtures.** Commit at most a few KB of real captured account bytes: one or two Whirlpool pool accounts and one TickArray. Store them with their slot and pubkey under `crates/whirlpool-core/tests/fixtures/`, and add **tests only** to `whirlpool-core` that decode them. Make no changes to `whirlpool-core` source. Label the fixtures as real captured public account data for decoder tests, not as evidence of opportunities.
7. **Runs.**
   - **(a) A 10-minute discovery run** with an empty TickArray allowlist, capturing pool accounts only. Use `capture_inspect` to build the inventory.
   - **(b) Choose the allowlist** from that inventory: every observed pool whose mint pair is SOL/USDC, SOL/USDT or USDC/USDT. The mints are wrapped SOL `So11111111111111111111111111111111111111112`, USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` and USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCE8BenwNYb`; verify all three against the decoded data. These pairs give both same-pair, multi-pool routes and SOL–USDC–USDT triangles. Commit the allowlist file.
   - **(c) A one-hour run** with that allowlist and the transaction stream on.
   - **Report for both runs:** updates per second by type, bytes per hour, projected GB per day, gap and stall counts, decode success rates and peak memory.
   - **Do not leave any process running when you finish.** Document the exact start and stop commands, without secrets, in the crate README. Whether capture runs continuously is the operator's decision.
8. **Validate:**
   - `cargo fmt --all`;
   - `cargo clippy -p geyser-capture -p whirlpool-core --all-targets -- -D warnings`;
   - `cargo test -p geyser-capture -p whirlpool-core`;
   - `cargo build --workspace --locked` after the lockfile is updated;
   - `./scripts/ci/repo_hygiene.sh`;
   - `dotnet test dotnet/MEVana.slnx --verbosity quiet`, which must be unchanged from 08;
   - `openspec validate --all --strict`;
   - `git diff --check`.

   Run `git grep` for the token's variable *values*, checking by length and prefix without printing them, to confirm that no secret was committed. Then archive the OpenSpec change.
9. **Write the closeout and push.** Write `docs/2026-10-XX-HHMM-CDT-research-handoff-09-closeout.md`. It must record:
   - the preflight results;
   - the volumes from both runs and the extrapolation;
   - the allowlist and how it was chosen;
   - decoder findings;
   - recommended rotation and retention settings, plus disk needs for 7 and 30 days;
   - open risks.

   Commit in logical units. Before pushing, fetch and merge any commits that exist only on the remote, without discarding anything. Push as a fast-forward, never force, and finish synchronized and clean.

## Constraints

- **Read-only.** No transactions, signing, wallet access, executor or hotpath changes, Kafka, PostgreSQL or Docker changes, and no changes to MEVana-UI or the .NET projects.
- Do not modify, delete, rotate or create credentials, `.env` files or wallet material. Never log or commit tokens.
- Rust changes are limited to the new `geyser-capture` crate, test-only additions to `whirlpool-core`, the workspace `members` entry and the resulting `Cargo.lock` additions.
- Do not commit capture data other than the decoder fixtures in step 6.
- Keep trading disabled. Date every document, and make sure no work is lost.

## Later slices (not in scope)

- **Handoff 10 — Replay correctness:**
  - fix the `arb_replay` problems in finding F8: the 1,024-signal queue loss, historical clock semantics and seed control;
  - add a step that converts raw captures into the replay input, the `EdgeDelta` frames from `crates/edge-assembler/proto/edge.proto`. `arb_replay` currently replays derived edges, not raw Geyser updates;
  - write the first `Simulated` `ExperimentResult` from a real capture.
- **Handoff 11:** a read-only Research API that serves the catalog, including capture-backed datasets, and the contract artifacts.
- **Handoff 12:** the MEVana-UI research page.
- **Follow-up:** corrections to the request contract, and Raydium decoders with tests once Whirlpool capture is stable.

## Exit criteria

Either the preflight failed and the closeout states the blocker precisely, **or** all of the following are true:

- `geyser-capture` is specified, implemented and tested.
- One-hour capture files exist under `~/MEVanaData/captures/` with valid manifests, and every gap is recorded.
- `capture_inspect` validates those files and reports Whirlpool decode rates.
- `whirlpool-core` has its first passing decoder tests, using real fixtures.
- No secrets or bulk data are committed, and no process is left running.
- Local and GitHub `main` are synchronized.

Captured data must not be described as evidence that arbitrage is profitable; it is input for later replay.
