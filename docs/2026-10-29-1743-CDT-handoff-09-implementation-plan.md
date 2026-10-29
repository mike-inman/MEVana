# 2026-10-29 17:43 CDT — Handoff 09 implementation plan

Created: 2026-10-29 17:43 CDT

Implement the revised 2026-10-29 handoff in this dedicated worktree on feat/geyser-capture-20261029. Entry main e30ceaea equals origin/main after fetch; committed handoff 08 closeout exists and five specs pass strict validation.

1. Narrow OpenSpec contract-only restrictions and specify market-data-capture.
2. Run one owner-filtered, Processed, read-only account preflight with a 60-second hard stop. Read only the authorized Geyser keys from the primary checkout root .env, without copying that file into this worktree. If connection or authentication fails, stop implementation and commit a blocker closeout as required by the handoff.
3. On preflight success, build the isolated crate, IDL-derived filters, durable raw frames, manifests, inspection and metrics; correct only the permitted Whirlpool decoder layout and length check.
4. Run offline tests, five-minute discovery and bounded allowlist validation; commit only public decoder fixtures and the observed-pair allowlist.
5. Complete handoff validation and closeout, archive only completed work, and synchronize via fast-forward Git operations without discarding concurrent changes.

No trading, wallet access, secret edits, deployment, factory jobs or changes to existing service implementations. Retain incomplete OpenSpec tasks if preflight blocks implementation.
