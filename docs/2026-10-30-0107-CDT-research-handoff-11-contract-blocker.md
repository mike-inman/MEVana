# 2026-10-30 01:07 CDT — Handoff 11 contract blocker

## Purpose and disposition

Handoff 11 stopped before implementation. The existing canonical `ExperimentRequest` cannot honestly identify the required candidate universe of both same-pair routes and SOL/USDC/USDT triangles. The handoff explicitly requires stopping if the contract cannot express the result, and forbids changing existing .NET contract behavior in this slice.

## Evidence

1. Handoff 11 design requirement 4 requires all ordered same-pair 2-leg routes and both SOL→USDC→USDT→SOL triangle directions over the standard-fee allowlist. A triangle necessarily uses one SOL/USDC pool, one USDC/USDT pool and one SOL/USDT pool. The inspected allowlist contains all three pair types; for example `21gTfx...` is SOL/USDC, `4fuUiY...` is USDC/USDT and `AhhoxZD...` is SOL/USDT.
2. `ExperimentRequestSyntaxValidator` requires `tokenPair` to contain exactly two distinct identifiers and `poolIds` to contain 2–16 identifiers. `ExperimentRequestValidator` requires every listed pool to contain **both** tokens in the sole `tokenPair`. The baseline OpenSpec `research-experiment-request` specification states the same semantics.
3. `ExperimentRequestCanonicalizer` fingerprints exactly that pair and pool list. `ExperimentResultValidator` can match a result to the request fingerprint, dataset, strategy and window, but does not make an omitted cross-pair pool part of the request. A result could therefore pass its validator while citing a request that excludes pools actually evaluated. That would be misleading evidence, so validation alone is insufficient.

Choosing SOL/USDC as the request pair excludes the SOL/USDT and USDC/USDT triangle pools. Choosing either other pair has the same problem. Splitting into per-pair requests cannot represent any triangle. Omitting the other pools from `poolIds` while replaying them misstates the canonical request scope. Restricting the replay to one pair violates handoff 11's route requirement.

## Work state

- Created the requested isolated worktree and branch from `origin/main` at `4ff6dd71`.
- Confirmed `~/.cargo/bin/cargo -V` reports `cargo 1.92.0-nightly (24bb93c38 2025-09-10)`.
- Read the full handoff and inspected the allowlist manifests, current Rust quote/frame interfaces, C# request/result code and baseline OpenSpec requirements. No capture replay, provider connection or trade action ran.
- No OpenSpec change, replay crate, request, result, policy comparison or determinism hashes were created. No validation suite was run because there is no implementation to validate. Existing source and capture data remain untouched.

## Recommended decision

Authorize a separate, versioned research-request contract change that can explicitly name a multi-pair route universe and its quote asset, with canonicalization, catalog validation and tests updated together. Then revise handoff 11 to depend on that contract version. Preserve the existing version-one request behavior for existing callers. After that, replay can produce a request fingerprint whose scope truthfully includes all evaluated pools and both route shapes.

This report is a contract finding, not an assertion that the capture is too thin or that any policy is profitable.
