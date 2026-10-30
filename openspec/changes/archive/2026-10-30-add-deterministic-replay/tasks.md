# Tasks — 2026-10-30 CDT

## 1. Offline replay core

- [x] 1.0 Bound the exact wrapper at supplied array edges and verify the two-direction 10-second regression; commit `557e42b5` separately.
- [x] 1.1 Add the opt-in crate and verify it compiles with pinned rustup cargo offline.
- [x] 1.2 Verify manifests, order captured writes, reconstruct pool/tick state and gaps; pass synthetic ordering and staleness tests.
- [x] 1.3 Generate 30/20/2 aligned ordered routes and exact-size quotes; pass synthetic route, direction, fee-unit and cost tests.

## 2. Policies and artifacts

- [x] 2.1 Implement the three policy buckets, future-slot simulation and checked accounting; pass synthetic policy and estimated/simulated separation tests.
- [x] 2.2 Emit stable candidate JSONL, summaries, canonical request/result pairs and run manifest; verify same input produces matching output hashes.

## 3. Contract and evidence

- [x] 3.1 Add capture-backed test catalog and three bounded default fixtures; verify all three requests/results with existing C# validators.
- [x] 3.2 Run default all-pair replay twice, plus pair A L=2 and cost sensitivity; verify hashes, counts and honest rejection shares.
- [x] 3.3 Write dated closeout with parameters, per-pair policy comparisons, limitations and next steps; verify it matches run summaries.

## 4. Integration validation

- [x] 4.1 Pass handoff step 7 format, Clippy, Rust tests, .NET tests, OpenSpec, hygiene, diff and non-printing secret scan on pinned toolchain.

## Workflow follow-up

- Archive the completed OpenSpec change and verify its synced spec.
- Commit logical units, push the branch, verify clean synchronized state and leave no process running.
