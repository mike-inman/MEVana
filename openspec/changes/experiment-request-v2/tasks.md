# Tasks — 2026-10-30 CDT

## 1. Contract

- [ ] 1.1 Add raw v2 DTO and syntax validator; verify mirrored v1 boundary and preservation tests plus v2-specific codes.
- [ ] 1.2 Add deterministic route enumerator and v2 catalog validator; verify R1=120, R2=152, R3 unused_pool and catalog failure cases.
- [ ] 1.3 Add v2 canonicalizer and fingerprint overload; verify golden bytes/hashes, sorting, culture and invalid-input cases.
- [ ] 1.4 Add v2 result linkage overload without v1 behavior changes; verify version separation and all handoff 11 v1 fingerprints.

## 2. Completion

- [ ] 2.1 Run dotnet build/test, strict OpenSpec validation, repo hygiene, diff check and non-printing secret scan; write a dated closeout with evidence.

## Workflow follow-up

- Sync specs and archive the completed OpenSpec change.
- Commit and push the clean synchronized feature branch; leave merging to the operator.
