# Tasks

## 1. Offline capture and decoder foundation

- [x] 1.1 Add the `gap-survey` crate with a pool-account-only Confirmed request and hard byte/duration limits; verify mocked-stream budget tests and offline `cargo test -p gap-survey`.
- [x] 1.2 Pin published Whirlpool, Raydium CLMM and Meteora DLMM layouts; verify real-account fixtures, owner and mint checks, and same-pair price agreement tests before any live stream.
- [x] 1.3 Commit a reviewed allowlist of at most 40 pools with provenance and verified mints; verify all pool-account reads and source metadata.

## 2. Deterministic analysis

- [x] 2.1 Implement exact-decimal gap math, stale exclusions, event boundaries and volatility/move labels; verify hand-computed and boundary tests.
- [x] 2.2 Implement deterministic offline session analysis and fixed segment gate; verify two output runs are byte-identical and gate thresholds remain unchanged.

## 3. Research presentation

- [x] 3.1 Add additive `research` tables and transactional repeatable load script; verify safe reload on the local research stack.
- [x] 3.2 Add the provisioned Indicative dashboard and JSON checks; render all panels and verify real zeros and unknowns display correctly.

## 4. Bounded live survey and closeout

- [ ] 4.1 Complete the offline validation list and credential scan before live use; record outputs in a dated closeout.
- [ ] 4.2 Run one 300-second pilot, inspect update rates and project bytes; verify provider sockets close and stop if any venue is absent or projected bytes exceed budget.
- [ ] 4.3 Run at most six bounded main sessions across three times of day, analyze and load each; verify cumulative payload bytes stay within 1.5 GB and no process remains.
- [ ] 4.4 Record H1-H4, per-segment fixed gate outcomes, limitations, bytes, panel render checks and final validation in a dated closeout; verify documents and artifacts contain no secrets or raw captures.

## Workflow follow-up

- Archive the OpenSpec change after completion.
- Push the clean branch without merging.
