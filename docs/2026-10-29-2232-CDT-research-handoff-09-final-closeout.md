# 2026-10-29 22:32 CDT — Handoff 09 final closeout: bounded read-only Geyser capture

## Outcome and source

The five-minute discovery and signaled ten-minute allowlist capture completed on the Sodae Yellowstone trial, with Processed commitment and `connection_tier: dev-test`. Captures and full audits are under `~/MEVanaData/captures/handoff09-20261029/`, outside Git. The manifests identify `mainnet-beta (known mint inventory)` from decoded wrapped SOL, USDC and the corrected USDT mint. This is a cluster inference from public mint identities, not a provider attestation. These short samples validate recording and decoding; they do not establish feed completeness, production latency or profitable opportunities. Trading remained disabled.

The prior 20-second Sodae and Tatum preflights were not repeated. The already completed 60-second Sodae filter check accepted the datasize, memcmp and transaction filters and received 929,396 protobuf payload bytes. The older Shyft authentication blocker remains historical. Tatum was not used in this continuation; its free-plan quotas remain unknown.

## Runs and after-run audits

Both commands ran in a tracked supervisor session from `feat/geyser-capture-20261029`, with an enforced `--duration`, sampled RSS and a post-run process/socket audit. They opened no owner-wide subscription.

| Run | Recorder command (no secrets) | Stop | Raw payload | Finalized files | Peak sampled RSS |
| --- | --- | --- | ---: | ---: | ---: |
| Discovery | `target/debug/geyser-capture --provider sodae --duration 300 --output ~/MEVanaData/captures/handoff09-20261029 --transactions false` | Duration at 300.3 s; supervisor 301.6 s | 4,257,619 B | 1 | 18,546,688 B |
| Allowlist | `target/debug/geyser-capture --provider sodae --duration 600 --output ~/MEVanaData/captures/handoff09-20261029 --rotation-seconds 150 --allowlist crates/geyser-capture/allowlist.json` | Supervisor sent SIGINT at about 570 s; process finalized at 572.0 s, supervisor 573.1 s | 124,731,652 B | 4 | 22,241,280 B |

The supervisor commands were `python3 /tmp/mevana_capture_runner.py discovery 300 0 --transactions false` and `python3 /tmp/mevana_capture_runner.py allowlist 600 570 --rotation-seconds 150 --allowlist crates/geyser-capture/allowlist.json`. For manual operation, the crate README gives the exact recorder, inspector and `kill -INT CAPTURE_PID` commands. The final allowlist file was finalized from `.part` on signal; neither session has a remaining `.part` file. `capture_inspect` verified all file hashes, framing, sequence and manifest totals.

After each run, DNS resolved Sodae to `45.154.33.73` and Tatum to `34.160.43.88` / `2600:1901:0:3a76::`. The immediate audits found no capture or probe executable and no established provider socket. A repeat exact-name `pgrep` and provider-IP `lsof` check after inspection was also empty. The earlier filter-check audit saw a `Codex (Service)` browser keep-alive to the Sodae IP; Revision 3 clarifies that its measured roughly 3 B/s does not count as a capture stream or fail this check. It was absent in the two new after-run audits. No process started by this work remains running.

Sodae protobuf payload total for the slice is **129,918,667 B**: 929,396 filter check + 4,257,619 discovery + 124,731,652 allowlist. Tatum total is **0 B** in this continuation. This is below the 0.5 GB slice cap; HTTP/2, TLS and TCP overhead and provider-billed volume are unknown. No provider reported quota exhaustion or rejected filters.

## Inventory, decoding and rates

Discovery produced 4,765 pool updates (15.80/s), 4,161 slot updates (13.80/s), 411 distinct pools, one recorded account-slot discontinuity (463130019 to 463130024), zero stalls, and 4,765/4,765 successful pool decodes. It used 846,920 payload B/min. Every observed pool whose mint pair was SOL/USDC, SOL/USDT or USDC/USDT is in the committed 18-address `crates/geyser-capture/allowlist.json`. The mints are wrapped SOL `So11111111111111111111111111111111111111112`, USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`, and USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`. Each was present in decoded inventory.

The allowlist run produced 10,902 pool updates (19.02/s), 6,242 fixed TickArray updates (10.89/s), 496 dynamic TickArray updates (0.87/s), 7,949 slot updates (13.87/s) and 4,946 transaction updates (8.63/s). Its transaction filter retained failed as well as successful non-vote transactions; this capture does not evaluate transaction outcomes. All 10,902 pools and 6,242 fixed arrays decoded successfully. Dynamic arrays were captured and counted as unsupported by the fixed decoder. There were zero other unexpected account types or lengths, eight recorded account-slot discontinuities, zero stalls, and 13,057,905 payload B/min. The gap details are in the four per-file manifests. Gaps and dev/test behavior mean neither run proves complete account history.

The committed compressed fixtures contain one real public USDC/USDT pool (slot 463130589) and a matching fixed TickArray (slot 463131460). `whirlpool-core` now decodes its IDL-correct 9,988-byte layout and rejects wrong lengths; the fixture test checks the pool key, mint pair, plausible tick index and initialized ticks. Only `whirlpool-core/examples/seed_and_log.rs` and the new inspector call `decode_tick_array`; the existing caller benefits from the fix and was not edited. Dynamic decoding and exact quote math remain later work. The other preflight account families were identified from discriminators and Orca source in `docs/2026-10-29-2148-CDT-geyser-account-layouts.md`: Whirlpool 653 B, fixed TickArray 9,988 B, DynamicTickArray 148+112·n B, Position 216 B and Oracle 254 B.

## Volume and retention planning

The discovery sample projects roughly 50.8 MB payload/hour or 1.22 GB/day; the allowlist sample projects roughly 783 MB payload/hour or 18.8 GB/day. The latter's actual framed files were 125,407,798 B over 573.1 supervisor seconds, projecting about 18.91 GB/day, 132.34 GB for 7 days or 567.16 GB for 30 days. With 20% free-space headroom, plan about 159 GB or 681 GB respectively. These are rough linear projections from short dev/test runs, not provider billing estimates or production capacity guarantees.

For the validated bounded run, 150-second time rotation and a 64 MiB size ceiling produced several independently verifiable files. A future continuous recorder can use that starting point and choose a retention period after measuring sustained volume and disk capacity. No automatic deletion or continuous stream was started here.

## Validation and limitations

- Pinned `nightly-2025-09-15` toolchain: 14 capture offline tests plus the real Whirlpool fixture test passed; focused Clippy (`-D warnings`) passed; targeted crate build passed.
- `cargo fmt --all` ran. It reformatted six unrelated previously clean files; those formatter-only changes were restored. Scoped `cargo fmt -p geyser-capture -p whirlpool-core -- --check` passed. The workspace-wide formatting check still reports only pre-existing out-of-scope differences.
- `cargo build --workspace --locked` remains blocked by the pre-existing `dex-registry`/`rdkafka-sys` OpenSSL link symbols `_BIO_eof` and `_OPENSSL_sk_set_thunks`. `cargo test --workspace --locked` likewise failed in `arb-engine` linking on those symbols, before a workspace test result. This does not affect the targeted recorder build or live runs; no unrelated crate was changed to work around it.
- `./scripts/ci/repo_hygiene.sh` passed with warnings about pre-existing dated docs and archived script locations. `dotnet test dotnet/MEVana.slnx --verbosity quiet` passed 194/194. `openspec validate --all --strict` passed 6/6. `git diff --check` passed. A non-printing scan of tracked files found zero full-token or 12-character-prefix matches for the two approved provider keys.

Open risks: dev/test completeness and local network slot gaps; unknown provider-billed bytes and Tatum quota; unsupported dynamic TickArray decoding; the unrelated workspace OpenSSL link failure. Historical follow-ups remain the wrong Jupiter ID, the 9898 port collision and the Redis 6379/6380 mismatch. No credentials, wallet material, bulk capture data, API, UI, database, infrastructure, executor or hotpath source were changed.

OpenSpec `add-geyser-market-data-capture` completed 15/15 tasks, synced 11 new requirements to the main `market-data-capture` spec, and was archived as `2026-10-29-add-geyser-market-data-capture`. Strict validation passed after archive.
