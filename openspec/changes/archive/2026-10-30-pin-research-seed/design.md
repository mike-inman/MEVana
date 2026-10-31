# Design — 2026-10-30 CDT

## Context

The store already loads eleven committed JSON seed files into memory before binding. Its current checks validate structure and selected relationships, but some valid-looking byte changes remain possible. The handoff's eleven SHA-256 values were recomputed against the committed seed and all match.

## Goals / Non-Goals

**Goals:** Bind the API to the exact approved seed bytes and preserve the specific existing tamper failure codes.

**Non-Goals:** Changing seed data, contract or HTTP behavior, parsing every summary field semantically, provider access, or protecting against a source-code change.

## Decisions

- Keep an ordinal, private static read-only file-to-digest map in `ResearchArtifactStore`. Pins live in compiled source, outside the mutable manifest.
- Run the pin loop after loading, validating and duplicate checking all manifest entries, immediately before `Load` returns. Existing malformed seed cases retain their earlier failure codes. Files omitted from a shortened manifest are still checked because the pin map, not the manifest, drives this loop.
- Read pins through the existing path-guarded `Read` helper and compare lowercase SHA-256. A mismatch names only the file and `seed_hash_mismatch`.
- Put six new temporary-copy tests in a new file because the operator explicitly prohibited edits to existing tests. Their local helper asserts the same failure code as the existing private `Fails` helper.

## Risks / Trade-offs

- [Approved seed update requires a code change] → Deliberate review gate; recompute and review all pin changes in a later handoff.
- [A changed source tree can change both bytes and pins] → This local loopback slice does not claim protection from source compromise.

## Migration Plan

No data migration. Rebuild and restart the local API with the pinned seed. Rollback is a code revert; existing seed bytes remain untouched.
