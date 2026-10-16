# 2026-10-15 23:10 CDT — Main synchronization coordinating handoff

## Purpose and starting state

Reconcile local `main` (`7b57c61021f8eed643544c74b58a8b93289eab33`) with GitHub `main` (`7bd165edc61c2657738940cba71bb115f37b896a`) without losing either history or changing local credentials. Work in `/Users/michaelinman/Repos/MEVana-worktrees/main-sync-20261015` on `integration/main-sync-20261015`. The original `/Users/michaelinman/Repos/MEVana` checkout remains on local `main`.

The branches have 351 local-only and 340 remote-only commits. A preview reports 128 conflicts. The [conflict inventory](2026-10-15-2310-CDT-main-sync-conflict-inventory.md) assigns each one to a single slice. The [original plan](2026-10-15-2303-CDT-main-branch-reconciliation-plan.md) records the initial safety decision.

## Handoff order and gates

1. [01 — History and credential review](2026-10-15-2310-CDT-main-sync-01-history-credential-review-handoff.md). Complete this before copying local-only content to the integration branch or publishing local-only history.
2. [02 — Runtime and code](2026-10-15-2310-CDT-main-sync-02-runtime-code-handoff.md): 16 conflicting paths.
3. [03 — Deployment and operations](2026-10-15-2310-CDT-main-sync-03-deployment-operations-handoff.md): 47 conflicting paths.
4. [04 — Documentation and archive](2026-10-15-2310-CDT-main-sync-04-documentation-archive-handoff.md): 65 conflicting paths.
5. [05 — Final integration and validation](2026-10-15-2310-CDT-main-sync-05-final-integration-validation-handoff.md). Merge the histories, verify the whole tree, and prepare an exact reviewable publication target.

Slices 02–04 own disjoint *conflict* paths. They may be completed sequentially as commits on the integration branch so each slice remains recoverable. One integration owner must review cross-slice behavior. Nonconflicting merge changes remain in scope for slice 05.

## Rules for every handoff

- Recheck `git status`, branch, worktree, both pinned commits, and remote refs before editing. Stop if the baseline changed; update the inventory before proceeding.
- Preserve API keys, local passwords, and other user data. Do not delete, overwrite, or quote credential values in documents, logs, commits, or chat. A credential concern is a publication gate, not authority to remove the credential.
- For each conflict, compare both branch versions and document the chosen behavior. Preserve distinct functionality where both versions matter. Do not resolve a category wholesale to one side.
- Commit each completed slice with only its owned paths and evidence. Do not use `git reset --hard`, `git clean`, force push, or prune worktrees. Do not modify the original local `main` checkout.
- Treat local tests, remote CI, and production operation as separate evidence. Publishing this handoff does not approve a deployment or live migration.

## Definition of done

The integration commit descends from both pinned `main` tips, all 128 conflicts have reviewed resolutions, nonconflicting changes are inspected, relevant checks pass or gaps are recorded, both worktrees are clean, and the exact remote target has been checked immediately before any `main` update. No merge or push to `main` has occurred at handoff preparation time.
