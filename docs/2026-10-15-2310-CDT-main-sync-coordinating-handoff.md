# 2026-10-15 23:10 CDT — Main synchronization coordinating handoff

## Purpose and starting state

Reconcile local `main` (`7b57c61021f8eed643544c74b58a8b93289eab33` at the original preview) with GitHub `main` (`7bd165edc61c2657738940cba71bb115f37b896a`) without losing either history or changing local credentials. Reconcile code in `/Users/michaelinman/Repos/MEVana-worktrees/main-sync-20261015` on `integration/main-sync-20261015`. Keep all plans, handoffs, reports, and closeouts in `/Users/michaelinman/Repos/MEVana/docs` on local `main`.

The branches have 351 local-only and 340 remote-only commits. A preview reports 128 conflicts. The [conflict inventory](2026-10-15-2310-CDT-main-sync-conflict-inventory.md) assigns each one to a single slice. The [original plan](2026-10-15-2303-CDT-main-branch-reconciliation-plan.md) records the initial safety decision.

## Handoff order and gates

1. [01 — History and credential review](2026-10-15-2310-CDT-main-sync-01-history-credential-review-handoff.md). Complete this before copying local-only content to the integration branch or publishing local-only history.
2. [02 — Runtime and code](2026-10-15-2310-CDT-main-sync-02-runtime-code-handoff.md): 16 conflicting paths.
3. [03 — Deployment and operations](2026-10-15-2310-CDT-main-sync-03-deployment-operations-handoff.md): 47 conflicting paths.
4. [04 — Documentation and archive](2026-10-15-2310-CDT-main-sync-04-documentation-archive-handoff.md): 65 conflicting paths.
5. [05 — Final integration and validation](2026-10-15-2310-CDT-main-sync-05-final-integration-validation-handoff.md). Merge the histories, verify the whole tree, and prepare an exact reviewable publication target.

Slices 02–04 own disjoint *conflict* paths. Commit resolved code/configuration paths sequentially on the integration branch and dated evidence documents in the canonical `MEVana/docs` directory on local `main`. One integration owner must review cross-slice behavior. Nonconflicting merge changes and subsequent documentation-only `main` commits remain in scope for slice 05.

## Rules for every handoff

- Recheck `git status`, branch, worktree, both pinned commits, and remote refs before editing. Stop if the baseline changed; update the inventory before proceeding.
- Preserve API keys, local passwords, and other user data. Do not delete, overwrite, or quote credential values in documents, logs, commits, or chat. A credential concern is a publication gate, not authority to remove the credential.
- For each conflict, compare both branch versions and document the chosen behavior. Preserve distinct functionality where both versions matter. Do not resolve a category wholesale to one side.
- Commit each completed slice with only its owned paths on the integration branch and its dated documentation in local `main`. Do not use `git reset --hard`, `git clean`, force push, or prune worktrees. Do not change code in the original local `main` checkout during reconciliation.
- Treat local tests, remote CI, and production operation as separate evidence. Publishing this handoff does not approve a deployment or live migration.

## Definition of done

The integration commit descends from both pinned `main` tips, all 128 conflicts have reviewed resolutions, nonconflicting changes are inspected, relevant checks pass or gaps are recorded, both worktrees are clean, and the exact remote target has been checked immediately before any `main` update. No merge or push to `main` has occurred at handoff preparation time.

## 2026-10-16 documentation location update

These eight reconciliation documents were moved from the integration worktree into the canonical local `MEVana/docs` directory. The pinned local SHA above remains the conflict-analysis baseline; handoff 05 must merge the then-current local `main` tip so the documentation commits are included. Refresh the merge preview if those commits introduce additional conflicts.

## 2026-10-16 refreshed-baseline and build gates

Before slice 02, record the current local and GitHub `main` SHAs, worktree status, divergence counts, and a fresh merge-conflict inventory. The original 128-path list is a starting checklist only. Account for untracked files in either checkout before marking it clean.

If handoff 01 returns `HOLD`, stop integration and publication of local-only history. Preserve all original commits and credentials locally; document the affected commit/path categories without values. Resume only after a reviewed remediation decision and a repeat credential review establish a publishable history.

After handoff 05, record a separate build baseline for the synchronized tree. Fix build failures before starting new feature changes. A successful history sync by itself is not evidence of a properly building version.

## 2026-10-16 credential-preservation direction

The owner directs that credential rotation be deferred. Preserve current local credentials and all distinct environment entries. When reconciling tracked files that contain credentials, use the newest verified value for a duplicate setting; do not replace a real value with a placeholder, print values, or remove keys. Back up any local environment file before combining it with another identified source, and never commit a real `.env` file. The canonical checkout's `.env` is Git-ignored and neither `main` tip tracks one, so there is no Git `.env` conflict to resolve at present. The local `.env.example` is the newer, fuller tracked template.

This direction permits local conflict preparation without one-by-one rotation. It does not turn handoff 01's `HOLD` into a publication `GO`: making a commit with old credential history reachable from GitHub remains blocked until the history review is resolved or a reviewed publication approach is chosen.
