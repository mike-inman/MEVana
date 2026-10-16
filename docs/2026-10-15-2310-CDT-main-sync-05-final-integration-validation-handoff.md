# 2026-10-15 23:10 CDT — Handoff 05: final integration and validation

## Purpose

After handoffs 01–04, create a merge commit that preserves both `main` histories and the reviewed resolutions. Validate the resulting repository before any update to GitHub `main`.

## Entry gate

Require a `GO` from handoff 01, committed clean results from 02–04, the [128-path inventory](2026-10-15-2310-CDT-main-sync-conflict-inventory.md), and a fresh check of GitHub `main`. If either pinned tip has changed, refresh the merge preview and path ownership before merging.

## Work

1. In the integration worktree, record the current branch, clean status, commit SHA, and remote refs. Compare the final slice versions with the two pinned tips and confirm each intentionally includes or accounts for both sides.
2. Start a non-fast-forward merge of the pinned local `main` commit with `--no-commit`. Resolve conflicts using the already reviewed integration versions **only after** verifying they match the committed slice results. Review any newly appearing or auto-merged paths; stop if the inventory no longer matches.
3. Review the entire proposed merge against both parents, including nonconflicting additions and deletions. Check for conflict markers, accidental credential exposure, lost executable behavior, and configuration drift.
4. Run targeted checks from slices 02–04 and broader repository checks justified by the merged tree. Record commands, pass/fail results, and limits in a dated `docs/` closeout. Commit the merge only after the index has no unresolved paths.
5. Verify both pinned tips are ancestors of the merge commit and both worktrees are clean. Recheck the exact GitHub `main` SHA, CI/review evidence, and publication target. Publish the reviewed integration branch first; update `main` only with an explicit, exact fast-forward operation after review. Never force push `main`.

## Exit criteria

The merge commit is reviewable, contains both histories and all resolved work, has a sanitized dated closeout, and has verified local and remote evidence. A published integration branch alone is not synchronization of `main`; report that distinction until both local and GitHub `main` point to the accepted commit.
