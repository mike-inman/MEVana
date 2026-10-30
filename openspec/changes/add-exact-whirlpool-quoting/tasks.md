# Tasks — 2026-10-29 CDT

## 1. Licensed dependency boundary

- [x] 1.1 Pin both Orca crates at `=1.0.4` without default features; verify lockfile entry licenses and no existing version changes.
- [x] 1.2 Add attribution and lockfile guard test; verify the test fails on disallowed `orca_*` versions.

## 2. Exact quote core

- [x] 2.1 Decode fixed and dynamic arrays into a common facade; verify a captured dynamic fixture and malformed-length test.
- [x] 2.2 Implement integer exact-in and exact-out adapters with typed adaptive-fee and missing-array errors; verify unit tests in both directions.
- [x] 2.3 Document the new API and legacy approximation boundary; verify README reflects caller behavior.

## 3. Offline golden evidence

- [x] 3.1 Implement `swap_cases` extraction from finalized captures with transaction, pre-state, lookup-address and vault rules; verify focused offline tests.
- [x] 3.2 Evaluate both quote modes, post-state, mismatch classes and threshold rules; verify a versioned corpus and coverage report outside Git.
- [x] 3.3 Commit at most 256 KB compressed of real cases meeting pool, direction and crossing coverage; verify exact-match regression test.

## 4. Integration validation

- [x] 4.1 Run handoff step 8 validation, including dependent builds/checks and non-printing secret scan; record outcomes in a dated closeout.
- [ ] 4.2 Confirm no disallowed files or data changed; verify clean diff and synchronization before publication.

## Workflow follow-up

- Archive and sync the OpenSpec change after completed tasks and passing validation.
- Commit logical units, push the branch, and report its synchronized state.
