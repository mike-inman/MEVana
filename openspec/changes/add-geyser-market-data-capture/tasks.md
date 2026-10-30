# 2026-10-29 — Geyser capture tasks

## 1. Required connectivity gate

- [x] 1.1 Record the already passed Sodae/Tatum preflight results without repeating the gate; preserve the superseded Shyft blocker as history.

Historical Shyft failure remains in the earlier closeout. Superseded gate: Sodae passed (683 ms connect, 208 ms first account, 1,403 updates/20 s); Tatum passed (423 ms, 410 ms, 2,212 updates/20 s). Neither gate is to be repeated.

## 2. Recorder and offline evidence (preflight already passed)

- [x] 2.1 Add isolated crate/workspace member and locked dependencies; verify lock diff only adds package, document IDL-derived sizes/offsets and pass filter tests.
- [x] 2.2 Add approved-key configuration, secret type/redaction and source classification; test provider pairing and same-key environment precedence, private-source rejection and Synthetic manifests; document CLI and safe startup logs.
- [x] 2.3 Implement byte-preserving framed writer, rotation, SHA-256 manifests and session provenance; pass round-trip, atomic rename, corruption, .part and hash tests; document format v1 and DatasetMetadata mapping.
- [x] 2.4 Implement mockable source, subscriptions, ping response, reconnect, jitter, stalls and slot gaps; pass mock gap/backoff tests and document semantics.
- [x] 2.5 Implement fixed-port loopback geyser_capture_ metrics and signal/duration finalization; test occupied-port failure and PortConfig loading before registering port 8121; document start/stop commands.
- [x] 2.6 Implement capture_inspect integrity checks, decode summaries and pool inventory; test corrupt/incomplete input rejection and unexpected account reporting; document usage.

## 3. Bounded live validation and decoder evidence

- [x] 3.1 Capture five-minute pool discovery with hard stop; inspect inventory, confirm mints/cluster, measure rates, volume and peak memory, choose and commit observed-pair allowlist.
- [x] 3.2 Correct permitted Tick/TickArray layout and exact-length rejection offline before live runs; then add compact real public-account fixtures with slot/pubkey, run decoder tests and document caller impact without caller edits.
- [ ] 3.3 Run allowlist capture with transactions, at most ten minutes, 2–3 minute rotation and signal shutdown before deadline; inspect all files, prove no remaining process and report rates, gaps, decoder success and memory.

## 4. Integration and closeout

- [ ] 4.1 Run required fmt, focused clippy/tests, workspace locked build/tests, hygiene, unchanged .NET tests, strict OpenSpec validation, diff and non-printing secret scans; record exact outcomes.
- [ ] 4.2 Deliver dated closeout with tier/cluster/limits, both run volumes and qualified projections, retention/disk needs, decoder findings and open risks; verify only permitted fixtures and allowlist are committed.

## Workflow follow-up

- Archive only after successful implementation and required evidence; preserve pending tasks on preflight failure.
- Commit logical units, fetch/reconcile without discarding changes, push the rebased feature branch only with --force-with-lease; never push main and verify clean synchronized state.

## Revision 3 implementation additions — 2026-10-29

- [x] 5.1 Identify 653, 9988, 148+112*n, 216 and 254-byte accounts from discriminators and IDL/current Orca source without provider calls; derive all offsets.
- [x] 5.2 Test bound provider profiles, TLS-only Sodae override, dynamic capture and unsupported decode reporting, reconnect limit (default 5), aggregate 0.5 GB limit and no owner-wide requests.
- [ ] 5.3 After offline tests pass run filter-support check before discovery; after every run record manifest bytes, resolved provider IPs, pgrep and lsof checks and per-provider totals.
