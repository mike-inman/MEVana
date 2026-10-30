# 2026-10-28 08:35 CDT — Handoff 09: Read-only Geyser market-data capture

## Purpose

Start recording real Solana market data so that MEVana can eventually answer the research question: does ranked route selection beat simple baselines on executable, cost-adjusted opportunities? Contracts cannot answer that, and evidence accrues only in calendar time, so capture starts now. It runs alongside the later API and UI slices.

This slice builds a **record-only** capture tool for Orca Whirlpool state, checks that its files are intact, and exercises the existing Whirlpool decoders against real bytes. It **does not** detect opportunities, simulate trades, replay data, or trade.

## Revision 2026-10-29 17:38 CDT

The operator changed three things on 2026-10-29. Where any older text in this document conflicts with them, they take precedence.

1. **Reading `.env` is allowed for Geyser credentials only.** See *Credentials* below.
2. **This is a dev/test Geyser connection.** It is not a production feed. See *Dev/test connection* below.
3. **Live runs are short, 5–10 minutes at most.** The one-hour run is removed. See step 7.

It also adds an *Execution model* section, which rules out the old AgentEngineering factory process.

## Revision 2 — 2026-10-29 21:15 CDT: provider is Sodae; Shyft is blocked

These changes override the *Credentials* section and the preflight gate below where they conflict.

- **Shyft is blocked and Sodae is now the primary provider.**
  - Shyft (`grpc.va.shyft.to`) returned UNAUTHENTICATED for **both** `X_TOKEN` and `SHYFT_GEYSER_X_TOKEN`. See `docs/2026-10-29-1745-CDT-research-handoff-09-closeout.md` on branch `feat/geyser-capture-20261029`. Treat Shyft as blocked until the operator renews its access.
  - The primary provider is now the **Sodae Yellowstone trial**: 5 GB for 7 days, as described in `docs/2026-10-29-2032-CDT-tatum-sodae-geyser-development-setup.md`.
  - Tatum (header `x-api-key`) is the secondary provider. Its key is not in `.env` yet.
- **Credential mapping.** The capture tool reads these keys from the process environment first, then from `.env`. The `.env` names contain dots, so they must be parsed explicitly; they cannot be exported as shell variables.
  - **Token:** `SODAE.IO_API_KEY`, sent in the `x-token` header.
  - **Endpoint:** `SODAE.IO_YELLOWSTONE_GRPC_URL` is plaintext `http://ams.rpc.sodae.io:10201`. Use the documented TLS endpoint **`https://ams.tls.sodae.io:443`** instead, and refuse plaintext endpoints.
  - **Pairing.** Endpoints and tokens are chosen together, as a named provider profile with `--provider sodae|tatum|shyft|custom`. They are never chosen by independent alias fallback.
  - **Auth header.** The header name is set per provider: `x-token` for Sodae and Shyft, `x-api-key` for Tatum.
  - Do not edit `.env`.
- **The preflight already passed on Sodae, on 2026-10-29.** A 20-second subscription filtered by `ORCA_WHIRLPOOL` owner, at Processed commitment, using bindings generated from the locally cached yellowstone-grpc-proto **4.2.1** protos:
  - connected in 683 ms; the first account arrived 208 ms later;
  - received 1,403 account updates (about 70/s); the last slot was 463,112,617;
  - returned no errors.

  The pinned 4.2.1 client protocol is therefore compatible with Sodae, and **no Yellowstone dependency upgrade is needed**. Do not adopt the version 14 crates used by Sodae's example. Re-run the preflight only if the provider or credentials change.
- **Observed account lengths** in those 20 seconds:

  | Size (bytes) | Updates | Interpretation |
  | --- | ---: | --- |
  | 653 | 317 | Whirlpool pool |
  | 9,988 | 364 | Fixed TickArray; matches the IDL calculation |
  | 148 + 112·n (260, 372 … 10,004) | many | Probably the newer **dynamic tick array** variant |
  | 216 | 60 | Probably Position accounts |
  | 254 | 87 | Unidentified |

  Before choosing filters, identify each of these by its discriminator against the IDL, or against Orca's current program source if the repo IDL predates dynamic tick arrays. The TickArray filter must cover **both** fixed and dynamic tick arrays for allowlisted pools. Dynamic arrays have a different `whirlpool` field offset, so derive it rather than assuming it. Decoding dynamic tick arrays may be reported as unsupported in this slice, but they must be captured and counted.
- **Quota budget.**
  - Subscribing to everything the owner holds, unfiltered, used about 5.1 MB per 20 s, roughly **0.9 GB/hour**, which would exhaust the 5 GB trial in about 5.5 hours. Pool accounts alone use about 0.04 GB/hour.
  - Never run an unfiltered owner subscription beyond preflight. The 5-minute discovery run (pools only) and the 10-minute allowlist run should together use well under 0.5 GB.
  - Report the bytes used by each run in the closeout.

## Execution model (not the old factory process)

This slice is implemented **directly in this repository** by an interactive coding-assistant session (Claude Code or Codex), working with the operator. It follows this repo's own OpenSpec workflow, as handoffs 07 and 08 did.

It does **not** use the AgentEngineering factory or controller process from 2026-10-07 to 2026-10-10. Specifically:
- no factory jobs, guided goals, D1–D7 roadmap packages, assignment profiles or remote-worker dispatch;
- no permitted-write-file lists, per-file source-token ceilings or hash-pinned slice documents;
- no Developer Agent or Reviewer Agent runs, and no controller evidence records.

The 2026-10-07 to 2026-10-10 slice specifications in `docs/` (Experiment Request API, Discovery Contract, Core Requirement Trace and similar) and the `AgentEngineering` repository are history only. They are not authorities for this slice. Do not write anything into the `AgentEngineering` repositories. The scope rules here are this document's *Constraints* section and the OpenSpec change created in step 1.

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
  - **The `whirlpool-core` TickArray layout is wrong. This was verified on 2026-10-28 against the repository's own IDL at `idls/whirlpool_idl.json`.**
    - The IDL defines `Tick` as `initialized: bool` followed by five `u128`/`i128` fields and `[u128; 3]`, which is 113 bytes. `TickArray` is `startTickIndex: i32`, then `ticks: [Tick; 88]`, then `whirlpool: Pubkey`, which is 9,988 bytes including the 8-byte discriminator.
    - `whirlpool-core/src/types.rs:7-22` omits `initialized` and places `whirlpool` before `ticks`. Its own comment computes 9,900 bytes. Borsh does not require all input to be consumed, so `decode_tick_array` returns garbage without reporting an error.
    - `WhirlpoolMinimal` appears correct, but verify it too.
    - **Derive every data size and memcmp offset from the IDL, not from `whirlpool-core` types.** The expected values are a TickArray size of 9,988 bytes with the `whirlpool` field at offset 9,956 (8 + 4 + 88 × 113). Confirm the pool account size from the IDL as well. Then confirm all of these against the lengths of real captured accounts, and report any account types or lengths you see that you did not expect (for example a newer tick-array variant) instead of assuming.
- **Credentials (revised 2026-10-29).**
  - **Endpoint:** read from `GEYSER_ENDPOINT`. If that is unset, fall back to `SHYFT_GEYSER_URL`, then `ENDPOINT`.
  - **Token:** read from `GEYSER_X_TOKEN`. If that is unset, fall back to `X_TOKEN`, then `SHYFT_GEYSER_X_TOKEN` (as in `executor-service/src/config.rs:94`).
  - **Process environment first.** Values already exported in the environment take precedence over `.env`.
  - **`.env` is allowed for these keys only.** The capture tool and the preflight may read the repo-root `.env` (it is git-ignored), but only to load the keys named above. Checked on 2026-10-29, it contains `GEYSER_ENDPOINT`, `X_TOKEN`, `SHYFT_GEYSER_URL` and `SHYFT_GEYSER_X_TOKEN`. Two ways are allowed:
    - parse it in-process in the tool, using a dotenv crate only if one is already in `Cargo.lock`, or a minimal parser otherwise;
    - in a shell, export only those keys.

    Do not `source` the whole file, and do not load any other key from it.
  - **No other use of `.env`.**
    - Do not print, echo, `cat`, diff, copy, edit, rotate, create or commit `.env`, and do not include its values in logs, manifests, errors, test fixtures or documents.
    - Do not read the `.env.backup-*` files or any other secret file.
    - If you report on a key, report only whether it is present and its length or host. Never report its value.
- **Dev/test connection (acknowledged 2026-10-29).** The configured Geyser endpoint and token are a **dev/test connection**, not a production or paid low-latency feed.
  - **Expect limits.** There may be rate limits, connection caps, throttling, reduced filter support or occasional resets. Treat these as properties of the connection, and report them as findings, not code defects, unless the evidence shows otherwise.
  - **Validation only.** Results from this slice show that the capture pipeline, file format, manifests and decoders work on real bytes. They are **not** production benchmarks of latency, throughput, completeness or gap rate.
  - **Confirm the cluster.** Confirm from the decoded pool inventory, using the known mainnet mints in step 7b, which cluster the connection serves. Record that in the manifests and the closeout. If it is not mainnet-beta, say so prominently.
  - **Manifest fields.** Every manifest records `connection_tier: "dev-test"`, which is configurable, alongside the endpoint host. Capture-backed datasets carry that tier forward so that later replay results can be qualified accordingly.
- **The OpenSpec project context blocks this slice and must be narrowed.** Line 14 of `openspec/config.yaml` forbids live-data and provider calls and Rust changes. That restriction was written for the contract slices, but as project-wide context it would apply here.
- **A sweep of every crate (including `crates/_archive`) and `scripts/` on 2026-10-28 found no existing raw-stream recorder.**
  - Only the three files above subscribe to Geyser.
  - The `async_history_buffer` writes opportunities to the database.
  - The normalizer binaries (`tap_orca_*`, `arb_watch`, `swap_feed`) and `live_edge_audit` read Kafka.
  - `crates/multi-geyser-streams` contains only an empty `src/main.rs` today. Its last full version is at `git show 14874a0d^:crates/multi-geyser-streams/src/main.rs` (729 lines, deleted 2025-09-06).
    - That version was a transactions-only monitor: three parallel Processed subscriptions (Jupiter, Whirlpool, Raydium AMM), Prometheus `multi_geyser_stream_*` metrics, and a fixed 5-second retry.
    - It had no file output, no account subscription, no ping handling and no stall detection.
    - Its client and retry code continued as `hotpath_arb_engine/src/main.rs:4034` and `:4325`.

  Do not reuse any of these as a capture path.
- **Reuse two ideas from `multi-geyser-streams`:**
  1. Give the account stream and the transaction stream **separate connections**, so that transaction volume cannot starve account capture. Record gaps per connection.
  2. Expose a **Prometheus metrics endpoint**: updates per second by type, bytes written, current file, last slot, seconds since the last message, and gap and stall counts.
     - Use a new `geyser_capture_*` metric prefix. **Do not reuse `multi_geyser_stream_*`**, because `scripts/geyser_recovery.sh` acts automatically on those metrics.
     - Choose a port that does not conflict with any port listed in `CLAUDE.md`, and make it configurable.
     - Bind to `127.0.0.1` by default.

  Do not restore or modify the old crate, and do not touch the Grafana dashboards in this slice.
- **`crates/shared-core` rules for this slice.** Do not modify `shared-core`.
  - **Use** `shared_core::programs::ORCA_WHIRLPOOL` for the program ID.
  - **Do not use `shared_core::Config`.** It holds `x_token` in a type that derives `Serialize` and `Debug`, so logging it or writing it to a manifest would leak the token. Keep the token in a dedicated type with redacted `Debug` and no `Serialize` implementation, and test the redaction.
  - **Do not use `shared_core::port_manager::get_port_with_fallback`** for the metrics port.
    - It silently moves to another port when the requested one is busy, and writes `/tmp/mevana_port_allocations.json`.
    - The same module contains `kill_port_process`. Never call it.
    - Instead, bind a fixed, configurable port and **fail at startup if it is busy**.
  - **Metrics port:** the default is `8121`, inside the metrics range (`8120-8139`) defined in `port_manager.rs`.
    - Before choosing it, verify that it is free and not listed in `config/ports.toml` or in the ports section of `CLAUDE.md`.
    - Record it as `geyser_capture_metrics` under `[development]` in `config/ports.toml`, with a dated comment. `PortConfig` does not declare `deny_unknown_fields`, but add a test proving that `PortConfig` still loads with the new key. If it does not, leave `ports.toml` alone and document the port in the crate README instead.
- **Pre-existing problems to record in the closeout as follow-ups, without fixing them:**
  - **Wrong Jupiter ID.** `shared_core::programs::JUPITER_V6` holds `JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB`, which the old `multi-geyser-streams` code labeled Jupiter v4. The old v6 ID was `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`.
  - **Port 9898 collision.** `config/ports.toml` gives 9898 to `token_registry_service`, while MEVana-UI's `src/lib/config.ts` uses `127.0.0.1:9898` for executor admin.
  - **Redis port mismatch.** `config/ports.toml` lists Redis as 6379, but `CLAUDE.md` lists 6380.
- **Guard against the fake Geyser.** `scripts/development/start_fake_geyser.sh` runs a synthetic Geyser on `localhost:50051`, created as an ISP-throttling workaround.
  - The capture tool must **refuse** loopback, private or `localhost` endpoints unless `--synthetic-source` is passed.
  - When that flag is set, every manifest must record the source as synthetic, so the data is never cataloged as market data. Test this.
  - The ISP-throttling history also means this Mac's network may introduce gaps. Report gap rates honestly, and recommend a server host if they are material.
  - A dev/test endpoint is still a real remote market-data source, not a synthetic one, so it does not need `--synthetic-source`. Its manifests carry `connection_tier: "dev-test"` instead.
- `whirlpool-core/examples/seed_and_log.rs` fetches pool and TickArray accounts over RPC (`RPC_URL`). It can serve as a cross-check for decoder fixtures, but the fixtures themselves should come from captured bytes.
- Raydium is out of scope. Its CLMM decoder is private and untested, AMM v4 decoding is a stub, and there is no CPMM account decoder.

## Work

1. **Update OpenSpec and propose the change.**
   - Edit `openspec/config.yaml` so the "no live-data/provider calls, Rust…" restriction applies to **research contract changes** only. Add standing capture rules:
     - capture is read-only market-data recording;
     - no signing, wallets, transaction construction or submission;
     - tokens never appear in logs, files, manifests, errors or commits;
     - captured market data is never committed, apart from the small fixtures described in step 6.
   - Then create the OpenSpec change `add-geyser-market-data-capture` (capability `market-data-capture`) with a proposal, design, tasks and spec delta covering the requirements in steps 3 to 5. Validate it strictly.
2. **Run a connectivity preflight before writing the capture tool, against the dev/test connection.** Using the existing `geyser_probe` approach or a 60-second throwaway subscription, open one read-only account subscription filtered to the Whirlpool owner. Record:
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
     - Write to `*.part`, `fsync`, then atomically rename on rotation. Rotation happens by size or by time interval. The defaults are once per hour or the size cap, but both are configurable, so the short live runs in step 7 can set an interval of 2–3 minutes and exercise rotation against real data.
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
   - **Configuration.** Use command-line options or environment variables for:
     - the output directory, rotation size and interval, commitment, allowlist file, transaction flag and stall timeout;
     - the connection tier;
     - a **maximum run duration** (`--duration`), after which the tool finalizes its files and manifests and exits cleanly. Every live run in this slice must set it.
     - Default the output to `~/MEVanaData/captures/`, which is outside the repository and not `/tmp`. Add the relevant ignore patterns anyway.
     - On startup, log the configuration with secrets redacted.
4. **Write a `capture_inspect` binary in the same crate.** It must:
   - check frame integrity and manifest hashes, and report `.part` files and gaps;
   - decode captured pool and TickArray accounts with `whirlpool-core`, reporting decode success and failure counts by account type;
   - produce a **pool inventory**: pool address, mint A and B, tick spacing and fee rate (as decoded), and update count.
5. **Tests** (no network): frame round trip, rotation and atomic rename, `.part` handling, manifest hash correctness, gap recording, backoff bounds, filter offsets and sizes, and secret redaction. For gap recording and backoff, use a mock update source, and introduce a source trait if needed.
6. **Decoder fixtures.** Commit at most a few KB of real captured account bytes: one or two Whirlpool pool accounts and one TickArray. Store them with their slot and pubkey under `crates/whirlpool-core/tests/fixtures/`, and add tests to `whirlpool-core` that decode them.
   - **Correct the `Tick` and `TickArray` structs in `whirlpool-core/src/types.rs` to match the IDL.** Make no other `whirlpool-core` source changes. Leave `math.rs`, `quoter.rs` and `tick_window.rs` alone; they are a known rewrite for a later slice.
   - Make `decode_tick_array` reject input whose length differs from the expected account size, so it cannot silently mis-decode.
   - Test both: a real TickArray fixture must decode with plausible values (`start_tick_index` divisible by `tick_spacing × 88`, and a `whirlpool` field equal to its pool), and wrong-length input must fail.
   - List every crate that calls `decode_tick_array` and say whether the fix affects it. Do not change the callers. Label the fixtures as real captured public account data for decoder tests, not as evidence of opportunities.
7. **Runs (revised 2026-10-29: short runs only, 5–10 minutes at most each).**
   - Every live run uses `--duration` (or an equivalent hard stop) so it cannot overrun.
   - No run in this slice lasts longer than 10 minutes. Longer or continuous capture is the operator's later decision.
   - **(a) A 5-minute discovery run** with an empty TickArray allowlist, capturing pool accounts only. Use `capture_inspect` to build the inventory.
   - **(b) Choose the allowlist** from that inventory: every observed pool whose mint pair is SOL/USDC, SOL/USDT or USDC/USDT. The mints are wrapped SOL `So11111111111111111111111111111111111111112`, USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` and USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCE8BenwNYb`; verify all three against the decoded data. These pairs give both same-pair, multi-pool routes and SOL–USDC–USDT triangles. Commit the allowlist file.
   - **(c) A 10-minute allowlist run** with that allowlist and the transaction stream on.
     - Set the rotation interval to 2–3 minutes, so the run produces several finalized files plus their manifests.
     - At the end, stop the run early with SIGINT or SIGTERM, before `--duration` expires, and confirm that shutdown finalizes the current `.part` file cleanly. If that is impractical, kill a separate short run of 1–2 minutes so that `capture_inspect` sees a real `.part` file.
   - **Report for both runs:** updates per second by type, bytes per minute, gap and stall counts, decode success rates and peak memory.
     - Extrapolate to bytes per hour and GB per day, and label those figures as rough projections from 5–10 minute samples on a dev/test connection.
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
   - the volumes from both runs and the extrapolation, with dev/test and short-sample caveats;
   - the connection tier, the observed cluster, and any dev/test limits encountered (rate limits, throttling, resets, unsupported filters);
   - the allowlist and how it was chosen;
   - decoder findings;
   - recommended rotation and retention settings, plus disk needs for 7 and 30 days;
   - open risks.

   Commit in logical units. Before pushing, fetch and merge any commits that exist only on the remote, without discarding anything. Push as a fast-forward, never force, and finish synchronized and clean.

## Constraints

- **Read-only.** No transactions, signing, wallet access, executor or hotpath changes, Kafka, PostgreSQL or Docker changes, and no changes to MEVana-UI or the .NET projects.
- Do not modify, delete, rotate or create credentials, `.env` files or wallet material. Reading `.env` is allowed only for the Geyser keys listed under *Credentials*. Never log, print or commit tokens or endpoint query strings.
- No live run longer than 10 minutes, and no process left running.
- Use the direct-implementation model in *Execution model*. Do not use the AgentEngineering factory or controller process.
- Rust changes are limited to the new `geyser-capture` crate; the `whirlpool-core` `Tick`/`TickArray` layout fix, the length check and the tests; the workspace `members` entry; and the resulting `Cargo.lock` additions.
- Do not commit capture data other than the decoder fixtures in step 6.
- Keep trading disabled. Date every document, and make sure no work is lost.

## Later slices (not in scope)

The sequence below was revised on 2026-10-28 after `docs/2026-10-28-0832-CDT-rust-crates-inventory-and-capability-map.md` found that no crate contains exact quote math.

- **Handoff 10 — Exact Whirlpool quoting:** an integer port of Orca's swap math, including tick crossing and `liquidity_net`. Golden-test it against captured on-chain swaps and the pool state before and after each one.
- **Handoff 11 — Replay correctness:**
  - fix the `arb_replay` problems in finding F8: the 1,024-signal queue loss, historical clock semantics and seed control;
  - add a step that converts raw captures into the replay input, the `EdgeDelta` frames from `crates/edge-assembler/proto/edge.proto`. `arb_replay` currently replays derived edges, not raw Geyser updates;
  - write the first `Simulated` `ExperimentResult` from a real capture.
- **Handoff 12:** a read-only Research API that serves the catalog, including capture-backed datasets, and the contract artifacts.
- **Handoff 13:** the MEVana-UI research page.
- **Follow-up:** corrections to the request contract, and Raydium decoders with tests once Whirlpool capture is stable.

## Exit criteria

Either the preflight failed and the closeout states the blocker precisely, **or** all of the following are true:

- `geyser-capture` is specified, implemented and tested.
- Short capture files (5-minute discovery run and 10-minute allowlist run, both against the dev/test connection) exist under `~/MEVanaData/captures/` with valid manifests that record `connection_tier`, and every gap is recorded.
- `capture_inspect` validates those files and reports Whirlpool decode rates.
- `whirlpool-core` has its first passing decoder tests, using real fixtures.
- No secrets or bulk data are committed, and no process is left running.
- Local and GitHub `main` are synchronized.

Captured data must not be described as evidence that arbitrage is profitable; it is input for later replay. Data from the dev/test connection must also not be described as representative of production feed latency or completeness.
