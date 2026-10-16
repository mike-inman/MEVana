# 2026-10-16 11:49 CDT — Handoff 06: Rust build remedy

## Purpose

Establish a verified, repeatable Rust build on the synchronized `main` tree before feature work. The earlier integration validation hit `ethnum` 1.5.2 error E0512 under a different Rust toolchain. Preserve the lockfile and application behavior unless a demonstrated failure requires a narrow change.

## Entry state

- Local and GitHub `main` both identify the reviewed snapshot commit `3367b550b569a53a4cb8ec57d289c0aa54be0b1b`.
- The local worktree is clean. The pre-snapshot history is retained privately in branch `backup/main-before-snapshot-20261016` and a verified local Git bundle.
- On this Mac, the active toolchain reports `rustc 1.92.0-nightly (52618eb33 2025-09-14)` and `cargo 1.92.0-nightly`.
- `cargo check --locked --offline -p ethnum` and `cargo check --locked --offline --workspace -j 4` pass on that toolchain. These checks do not establish that a full build or tests pass.

## Work

1. Record the exact Rust toolchain, available disk space, and the first failing command. Run `cargo build --locked --offline --workspace -j 4`. If it passes, do not modify dependencies merely to address the earlier non-reproducing E0512 error.
2. If the build fails, identify the first actual root cause, make the smallest source, dependency, or toolchain change needed, and keep the lockfile deterministic. Do not change trading logic or production deployment configuration as a shortcut.
3. Run focused checks for any changed crate. Then rerun the locked workspace build. Run a bounded workspace test pass if the build succeeds; distinguish compilation, unit tests, and tests requiring live infrastructure.
4. Document commands, exact pass/fail outcomes, toolchain sensitivity, and any remaining limits in a dated closeout in `docs/`. Commit the handoff, remedy, and closeout. Recheck the exact GitHub `main` target before a normal fast-forward push; never force push.

## Exit criteria

The repository has a passing locked Rust workspace build on a named toolchain, or a precise remaining blocker with a reproducible command. Any source change is reviewed and tested. Local and GitHub `main` are synchronized after the committed closeout. A passing check alone must not be described as a passing build.
