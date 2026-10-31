# Tasks — 2026-10-30 CDT

## 1. Pin approved seed

- [x] 1.1 Recompute all eleven SHA-256 values against the committed seed and stop on any mismatch; record the matching count.
- [x] 1.2 Add the ordinal compiled pin map and last-step verification to `ResearchArtifactStore.Load`; verify startup loads the unchanged seed.
- [x] 1.3 Add six new temporary-copy tamper tests without editing existing tests; verify each returns `seed_hash_mismatch`.
- [x] 1.4 Remove the pin loop temporarily, verify exactly the six new tests fail, restore it, and verify all 300 tests pass.
- [x] 1.5 Update the Handoff 15 closeout with the new failure code and pre-merge dependency; verify the dated document reflects the validation result.

## 2. Integration validation

- [x] 2.1 Run the required .NET, OpenSpec, hygiene and diff checks; verify no seed, Core, crates or existing fixture changes, no credential finding, and no port 8098 listener.

## Workflow follow-up

- Sync the research-api spec, archive this change, commit and push the branch; do not merge.
