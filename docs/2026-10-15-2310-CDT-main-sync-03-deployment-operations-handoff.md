# 2026-10-15 23:10 CDT — Handoff 03: deployment and operations conflicts

## Purpose and ownership

Resolve the 47 conflict paths assigned to slice 03 in the [inventory](2026-10-15-2310-CDT-main-sync-conflict-inventory.md): `.env.example`, `.gitignore`, `docker-compose.local.yml`, `.github/**`, `config/**`, `infrastructure/**`, and `scripts/**`. The only content conflict is `.gitignore`; the other 46 are add/add. Start only after handoff 01 permits local content to be incorporated.

## Work

1. Compare each side's deployment, startup, monitoring, backup, and test-script behavior. Keep environment examples free of real credential values. Preserve both useful operational paths where they serve distinct environments.
2. Reconcile `.gitignore` carefully so local credentials, generated files, and backups remain protected. Do not remove a local password or API key to make a merge easier.
3. Produce reviewed final versions on the integration branch and a dated resolution note under the canonical local `MEVana/docs` directory. Call out changed defaults, ports, service names, data volumes, CI triggers, and any action with live effects.
4. Run non-destructive syntax and configuration checks appropriate to the touched files. Use dry-run/what-if where available; do not deploy, rotate credentials, purge data, or invoke live maintenance scripts under this handoff.
5. Commit only owned conflict paths on the integration branch and commit the resolution note on local `main`. Record any validation requiring a later environment or approval.

## Exit criteria

All 47 owned paths are reviewed and committed, credential handling is intact, non-destructive checks and gaps are recorded, and the worktree is clean. The final merge is deferred to handoff 05.

## 2026-10-16 credential and environment merge direction

Defer credential rotation. Preserve the newest verified credential value for duplicate settings and retain distinct settings from both sources. If an actual second `.env` source is identified, make a protected backup and combine entries without printing values; keep `.env` out of Git. At present neither `main` tip tracks `.env`, so `.env.example` is the tracked conflict. The newer local `.env.example` contains the complete set of 27 assignment names while GitHub `main` contains a one-line comment; retain the full local template and account for the remote comment if it conveys an additional requirement. Local conflict work may proceed, but publishing local-only history remains subject to handoff 01's `HOLD`.
