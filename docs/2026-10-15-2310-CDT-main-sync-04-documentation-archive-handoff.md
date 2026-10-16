# 2026-10-15 23:10 CDT — Handoff 04: documentation and archive conflicts

## Purpose and ownership

Resolve the 65 conflict paths assigned to slice 04 in the [inventory](2026-10-15-2310-CDT-main-sync-conflict-inventory.md): `docs/**`, `_archive/**`, and root `CLAUDE.md`. Start only after handoff 01 permits local content to be incorporated.

## Work

1. Compare both versions of each path. Keep dated historical records distinguishable from current instructions. Preserve useful information from both sides; do not silently replace an archive or duplicate a document without noting why.
2. Keep plans and new handoff notes in the canonical local `MEVana/docs` directory with date, time, and purpose in the title. Check internal links and references that moved during reconciliation.
3. Treat archived shell scripts as executable risk despite their location. Do not run deployment, cleanup, or credential-changing scripts while resolving text conflicts.
4. Commit the resolved archived/doc paths required by the merge on the integration branch, and commit a dated resolution note in local `MEVana/docs`. The note records decisions, retained copies, and any unresolved factual claims needing later verification.

## Exit criteria

All 65 owned paths are reviewed and committed, the document index and links are checked where practical, no credential values were introduced into new documentation, and the worktree is clean. The final merge is deferred to handoff 05.

## 2026-10-16 additional conflict

The refreshed preview adds `docs/2026-10-15-vultr-hetzner-geyser-latency-decision-plan.md` as a 66th slice-04 path. Compare the local dated implementation plan with the older GitHub version and retain their distinct historical purposes or reconcile their content before the final merge.
