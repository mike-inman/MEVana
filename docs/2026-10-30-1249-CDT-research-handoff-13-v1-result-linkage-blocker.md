# 2026-10-30 12:49 CDT — Handoff 13 v1 result-linkage blocker

Handoff 13 stops before completion because its required cross-version result code conflicts with the unchanged v1 validator.

`ExperimentResultSyntaxValidator.Validate` accepts only `requestCanonicalizationVersion == 1`. The existing `ExperimentResultValidator.Validate(result, catalog, ExperimentRequest)` overload first calls the existing result validator and returns immediately on a syntax issue. Therefore a version-2 result checked against a v1 request returns `requestCanonicalizationVersion/unsupported_canonicalization_version`; it cannot return `request/request_mismatch` through that v1 path. The handoff explicitly requires `request_mismatch` in both cross-version directions and separately forbids any v1 behavior change. Meeting both would require changing the v1 path's issue set or early-return behavior.

A new v2 overload can reject a v1 result with `request_mismatch` without touching v1. It cannot change the reverse behavior. The operator needs to choose either (1) accept the existing v1 syntax issue as the reverse rejection, while retaining the v1 freeze, or (2) authorize a v1 behavior change to add `request_mismatch` for version-2 results.

Partial work is preserved on the feature branch: OpenSpec planning artifacts, v2 DTO, validators, route enumerator, canonicalizer and new overloads, plus an unfinished new test file. The tests contain pending golden hashes and golden fixture files have not been created. One offline restore and build passed before the new test file was added; no final test suite or closeout validation was run. The OpenSpec change remains active and unarchived. No network, `.env`, Rust, or handoff 12-owned test file was touched.
