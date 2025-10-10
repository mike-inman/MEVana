# Rebuild Plan (2025-10-05)

## High-Level Reassembly Plan

1. **Baseline preservation**
   - current branch `restore-20250930` mirrors the server snapshot.
   - create a safety branch (`snap-shot-20250930`) or tag before any new changes.

2. **Core modules to restore (already done)**
   - `crates/flash-loan-providers`
   - `crates/executor-service/src`
   - `crates/hotpath_arb_engine/src`
   - `crates/wallet-vault`
   - confirm with `cargo check`

3. **Reintroduce improvements**
   - real Jito submission
   - edge staleness controls
   - PropertyStore defaults + runtime watchers
   - Prometheus metrics updates

4. **Documentation & knowledge**
   - integrate `_archive` docs from server
   - add a recovery/lessons file

5. **Incremental workflow**
   - change → `cargo fmt` → `cargo clippy` → `cargo check` → commit
   - tag after each milestone

6. **Context retention**
   - keep this document updated
   - after each session update `docs/operations/SESSION_NOTES.md`

## Detailed Plan by Section

### A. Flash-loan provider plugins
- verify crate builds (`cargo check -p flash-loan-providers`)
- document providers in `docs/architecture/FLASH_LOAN_PROVIDER_ARCHITECTURE.md`
- ensure registry exports metrics for executor
- commit/push

### B. Executor service
- ensure `crates/executor-service/src` matches server snapshot
- run `cargo check -p executor-service`
- add tests for provider plan selection
- commit/push

### C. Hotpath arb engine
- confirm baseline builds (`cargo check -p hotpath_arb_engine`)
- improvements:
  1. add `max_edge_staleness_ms` property (PropertyStore + FastEngine)
  2. implement live Jito submission in `jito_integration.rs`
  3. remove synthetic heuristics
- after each: `cargo fmt`, `cargo clippy -p hotpath_arb_engine`, commit
- update docs

### D. Wallet vault
- `cargo check -p wallet-vault`
- document configuration in deployment guide
- commit/push

### E. Cross-cutting tests
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- note failing areas for follow-up

### F. Documentation
- move `_archive` docs into `docs/_archive`
- add `docs/operations/RECOVERY_NOTES.md`
- keep live `docs/operations/REBUILD_PLAN.md` updated

## Operational Practices
- frequent commits (even WIP)
- use branches/stash for experiments
- persist docs with each session
- occasional `git bundle` or tar backups
- session notes: add to `docs/operations/SESSION_NOTES.md`

## Next steps
1. commit this plan
2. run `cargo check -p flash-loan-providers`
3. begin hotpath improvements (Section C)
4. update documentation and commit after each milestone
