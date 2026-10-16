# 2026-10-15 23:03 CDT — Main branch reconciliation plan

## Purpose

Bring the local and GitHub `main` histories together without discarding either branch's commits, local credentials, or configuration.

## Verified starting point

- GitHub `main`: `7bd165edc61c2657738940cba71bb115f37b896a`.
- Local `main`: `7b57c61021f8eed643544c74b58a8b93289eab33`.
- Common ancestor: `23662e0428b5647241b658beb55d0abedbf654a4`.
- GitHub has 340 commits absent from local `main`; local has 351 absent from GitHub `main`.
- Both working trees were clean at inspection. The local and GitHub tips contain the same `private.txt` blob; this does not establish that all history is free of credentials.
- A non-mutating merge preview reports 127 add/add conflicts and one content conflict. A direct push of local `main` cannot fast-forward GitHub `main`.

## Safe integration sequence

1. Preserve the two current tips as independent branches. Use `integration/main-sync-20261015`, based on GitHub `main`, for reconciliation.
2. Review local-only history for credentials and other private material before publishing any branch containing it. Keep local passwords and API keys intact; do not copy their values into reports or commits.
3. Reconcile the 128 conflicting paths by inspecting both versions and the intended behavior. Do not resolve all conflicts wholesale to either side.
4. Validate the merged tree with targeted build and tests, configuration checks, and a clean working tree. Record any validation gaps.
5. Recheck GitHub `main` immediately before publication. Push the integration branch for review, then advance `main` only after the merged result is reviewed and the exact target is confirmed. Never force push `main` for this reconciliation.

## Current state

At the time of this plan, the integration branch recorded the plan only. It had not merged the local-only commits or synchronized `main`.

## 2026-10-16 documentation location update

The plan, coordinating handoff, five slice handoffs, and conflict inventory now live in `/Users/michaelinman/Repos/MEVana/docs`. The pinned commit IDs above remain the original conflict-analysis baseline. Later documentation-only commits on local `main` must be included when the final merge is prepared.

## 2026-10-16 execution gates

- The 351/340 commit counts and 128-conflict inventory describe the original tips, not the live merge target. Before resolving conflicts, record the current local and GitHub tips, verify the remote ref, rerun the merge preview, and update path ownership for any changed conflicts. Preserve all untracked user files before claiming a clean checkout.
- A credential-review `HOLD` stops publication of local-only history. Record the finding without secret values and obtain a reviewed remediation decision. Keep the original history and local credentials intact; do not silently rewrite, delete, rotate, or push them.
- Record the merged tree's build and test baseline. Synchronization and a passing build are separate milestones. Do not use the synchronized tree as the starting point for new feature work until the agreed build checks pass or the user explicitly accepts a narrower baseline.
