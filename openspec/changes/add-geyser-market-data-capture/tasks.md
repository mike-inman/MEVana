# 2026-10-29 — Geyser capture tasks

## 1. Required connectivity gate

- [x] 1.1 Execute a bounded owner-filtered Processed preflight; record connect/first-account timing, update count and sanitized error class. On connection/auth failure deliver blocker closeout and stop before group 2.

Preflight result (2026-10-29 17:45 CDT): TLS channel ready in 323.577 ms; subscription returned grpc_unauthenticated after 0.482 seconds overall, zero messages/accounts. Required blocker closeout delivered in docs/2026-10-29-1745-CDT-research-handoff-09-closeout.md. Tasks below remain blocked and incomplete.

## 2. Recorder and offline evidence (requires successful preflight)

- [ ] 2.1 Add isolated crate/workspace member and locked dependencies; verify lock diff only adds package, document IDL-derived sizes/offsets and pass filter tests.
- [ ] 2.2 Add approved-key configuration, secret type/redaction and source classification; test alias precedence, private-source rejection and Synthetic manifests; document CLI and safe startup logs.
- [ ] 2.3 Implement byte-preserving framed writer, rotation, SHA-256 manifests and session provenance; pass round-trip, atomic rename, corruption, .part and hash tests; document format v1 and DatasetMetadata mapping.
- [ ] 2.4 Implement mockable source, subscriptions, ping response, reconnect, jitter, stalls and slot gaps; pass mock gap/backoff tests and document semantics.
- [ ] 2.5 Implement fixed-port loopback geyser_capture_ metrics and signal/duration finalization; test occupied-port failure and PortConfig loading before registering port 8121; document start/stop commands.
- [ ] 2.6 Implement capture_inspect integrity checks, decode summaries and pool inventory; test corrupt/incomplete input rejection and unexpected account reporting; document usage.

## 3. Bounded live validation and decoder evidence

- [ ] 3.1 Capture five-minute pool discovery with hard stop; inspect inventory, confirm mints/cluster, measure rates, volume and peak memory, choose and commit observed-pair allowlist.
- [ ] 3.2 Correct permitted Tick/TickArray layout and exact-length rejection, add compact real public-account fixtures with slot/pubkey, run decoder tests and document caller impact without caller edits.
- [ ] 3.3 Run allowlist capture with transactions, at most ten minutes, 2–3 minute rotation and signal shutdown before deadline; inspect all files, prove no remaining process and report rates, gaps, decoder success and memory.

## 4. Integration and closeout

- [ ] 4.1 Run required fmt, focused clippy/tests, workspace locked build/tests, hygiene, unchanged .NET tests, strict OpenSpec validation, diff and non-printing secret scans; record exact outcomes.
- [ ] 4.2 Deliver dated closeout with tier/cluster/limits, both run volumes and qualified projections, retention/disk needs, decoder findings and open risks; verify only permitted fixtures and allowlist are committed.

## Workflow follow-up

- Archive only after successful implementation and required evidence; preserve pending tasks on preflight failure.
- Commit logical units, fetch/reconcile without discarding changes, push only fast-forward and verify clean synchronized state.
