# 2026-10-29 — Read-only Geyser capture v1

## 2026-10-29 — Offline swap cases v1

`swap_cases CAPTURE_ROOT OUTPUT_ROOT [DYNAMIC_FIXTURE]` reads only the six finalized handoff 09 `.cap` files. Use `~/MEVanaData/captures/handoff09-20261029/` and `~/MEVanaData/golden/whirlpool-20261029/` respectively. The optional third argument writes one compressed real dynamic-array account fixture. No provider connection is made.

The output `cases-v1.jsonl` has one JSON object per evaluable candidate, with `version: 1`, base58 signature/pool, slot, direction, integer observed vault input/output, raw pre/post pool bytes, keyed raw tick-array bytes, both quote results, classification, and an initialized-tick-crossing flag. `report-v1.json` gives totals, per-pool classes, coverage and limited unexplained signature/slot examples. Both are public on-chain quote-correctness data, not evidence of opportunities. Keep the full corpus outside Git; only the bounded compressed subset under `whirlpool-core/tests/golden/` is committed.

This isolated tool records public Orca market data for later research. It contains no wallet, signing or submission path. Processed dev/test data is not a profitability result or production latency/completeness benchmark. Data stays in `~/MEVanaData/captures/`; only small public decoder fixtures belong in Git.

Provider profiles bind all three credential properties together:

| Profile | TLS endpoint | Approved key | Header |
|---|---|---|---|
| sodae | https://ams.tls.sodae.io:443 | SODAE.IO_API_KEY | x-token |
| tatum | https://solana-mainnet-grpc.gateway.tatum.io:443 | TATUM_MAINNET | x-api-key |

Environment values precede the same key in the primary repo-root `.env`. No alias fallback or Shyft profile exists. Never source `.env`. The plaintext Sodae configuration is ignored in favor of the TLS override. Endpoint overrides cannot cross profile hosts. Tokens have redacted Debug and no Serialize implementation. Synthetic loopback/private endpoints require `--synthetic-source`; real credentials are never attached to a synthetic connection. DNS addresses are checked before connections; TLS still verifies the hostname.

Run from the dedicated worktree after offline tests. All durations are seconds, positive and at most 600. The metrics bind is fixed and fails when occupied; it never kills a process or silently changes port.

```sh
cargo build -p geyser-capture --locked
# Filter acceptance only: at most 60 seconds. Do not repeat the preflight.
target/debug/geyser-capture --provider sodae --filter-check --duration 60
# Five-minute discovery: empty allowlist means pools and slots only, no transaction connection.
target/debug/geyser-capture --provider sodae --duration 300 --transactions false
# Ten-minute target run; use the observed inventory allowlist. Signal before the deadline.
target/debug/geyser-capture --provider sodae --duration 600 --rotation-seconds 150 --allowlist crates/geyser-capture/allowlist.json
# In the supervising session, target the recorded capture PID only:
kill -INT CAPTURE_PID
# Inspect each printed session path offline. No new stream is needed.
target/debug/capture_inspect /absolute/path/to/capture-SESSION
```

The account connection requests pools using datasize plus discriminator memcmp; each allowlisted pool adds one fixed TickArray filter (size + discriminator + pool offset) and one dynamic TickArray filter (discriminator + distinct pool offset). Slots include processed, confirmed and finalized status. A second connection requests non-vote transactions involving the selected pools; `failed` is unset to retain successes and failures. Empty allowlists never create tick or transaction subscriptions. Dynamic arrays are counted as unsupported for tick decoding. Position and Oracle accounts are excluded. Layout derivations and exact source links are in `docs/2026-10-29-2148-CDT-geyser-account-layouts.md`.

Each connection answers Yellowstone pings, reconnects with exponential jittered backoff capped at 30 seconds and stops after five consecutive failed reconnects (configurable). No-message timeout defaults to 30 seconds. Gaps identify the connection, reason and any slot range. Slot discontinuities use only monotonically increasing Processed slot notifications, so repeated confirmed/finalized statuses do not create false gaps. Gaps do not prove exact lost-update counts. The hard deadline also covers connection/reconnect waits. A shared 450 MB payload budget counts prior runs in the output root, leaving framing/transport headroom below the 0.5 GB slice ceiling. At most two bounded in-flight messages can overshoot the threshold. A lock refuses concurrent runs; an incomplete prior session blocks further streaming until inspected. Do not change output directories to evade this budget. SIGINT/SIGTERM stop both sources, drain queued records, finalize files and exit. A failed source closes both connections and returns failure with sanitized diagnostics.

Format v1 is uncompressed. Eight magic bytes `MEVCAP01` precede records. Every record is `u32 LE length`, `u8 kind` (0 raw / 1 gap), `u8 connection` (0 accounts / 1 transactions), `u64 LE sequence`, `u64 LE receive_utc_ns`, then payload. Length includes the 18-byte header, excludes its four-byte length prefix, and is capped at 16 MiB. Raw payloads are the exact received protobuf message bytes, preserved by a raw tonic decoder including unknown fields; they are never reconstructed with Prost. Gap payloads are JSON. Sequence increases across both connections and all rotated files. Summaries/decode operations occur after raw append and never suppress frames.

Files are exclusively created as `000000.cap.part`, fsynced and atomically renamed to `.cap`; file SHA-256 and summaries go into `.cap.json` via fsynced atomic rename, including a directory fsync. Default rotation is hourly or 64 MiB; bounded runs use 150 seconds. `session.json` lists finalized manifests. Crashes leave `.part` or missing manifests; inspector reports/rejects incomplete sessions, truncation, hash/count/sequence mismatch and orphan files. Never catalog incomplete sessions. No automatic retention deletion is implemented. Preserve source files for later replay; any retention policy is an operator decision.

Manifest `bytes_received` counts raw uncompressed protobuf payload bytes, excluding HTTP/2, TLS and TCP overhead; on-wire quota can be higher. File `bytes` includes framing, magic and gap records. Manifests record source classification, provider host only, dev/test tier, observed cluster, subscription filters, allowlist, crate/git identity, coverage, counters and explicit gaps. Known mainnet mints observed in decoded pool inventory support mainnet-beta classification; absence leaves cluster unknown. Synthetic sessions always remain Synthetic.

For a later `DatasetMetadata` conversion: stable `session_id` maps to `Id`, format_version to `MetadataVersion`, first/last receive timestamps to zero-offset UTC coverage, and a sanitized `capture-SESSION-dev-test` identifier to `Provenance`. Provenance never includes a host URL, slash or backslash. Connection tier carries forward. This tool does not write the catalog or bypass its completeness checks.

Metrics at `127.0.0.1:8121` expose `geyser_capture_updates_total` and sample-average `updates_per_second` by type, `bytes_written`, `current_file`, `last_slot`, `seconds_since_last_message`, `gaps_total` and `stalls_total`. The prefix is distinct from the old recovery scripts. No Grafana changes are needed.

Offline tests cover framing/unknown fields, rotation, hashes/corruption, truncation and `.part`, source-derived layouts/filter alternatives, mock gaps/stalls, bounded reconnect/backoff, occupied metrics port, PortConfig compatibility, provider pairing, secret redaction/precedence, and synthetic classification. `capture_inspect` streams files with a maximum record allocation; inventory memory grows with the observed pool set, not stream length.

Existing decoder callers: `whirlpool-core/examples/seed_and_log.rs` receives correct fixed-array ticks/pool identity and now rejects wrong lengths. The new inspector also uses it. No other Rust crate directly calls `decode_tick_array`; callers are unchanged. Dynamic decode and quote/math/tick-window correctness remain later work.

The 2026-10-29 filter check captured USDT mint `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`; Revision 3 of the handoff now uses this corrected value. The dated final closeout in `docs/` records both bounded runs and the observed 18-pool allowlist.
