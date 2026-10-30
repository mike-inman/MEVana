# 2026-10-30 11:18 CDT — Handoff 13: Experiment request contract v2 (multi-pair route universes)

## Purpose

The v1 `ExperimentRequest` names exactly one `tokenPair`, and every pool in it must contain both tokens. It cannot truthfully describe a SOL–USDC–USDT triangle. Handoff 11 stopped on exactly this (`docs/2026-10-30-0107-CDT-research-handoff-11-contract-blocker.md`).

This slice adds a **separate, versioned v2 request contract**. It can name:
- a multi-token route universe;
- its quote asset;
- the route shapes evaluated;
- the cost-model price-input pools.

v1 stays byte-for-byte unchanged.

This slice is **contract-only** (.NET plus OpenSpec):
- no Rust changes;
- no replay;
- no network;
- no triangle evaluation.

The triangle replay is a later handoff (14), after this one and handoff 12 have both merged.

**Runs in parallel with handoff 12.** Handoff 12 is Rust, plus new .NET test fixtures and catalog entries. Do not edit `CaptureReplayCatalog.cs`, `CaptureReplayResultTests.cs` or any `Fixtures/2026-10-30-replay-*` file, because handoff 12 owns them.

**Numbering (supersedes earlier lists, including the "Later slices" in handoff 12):**

| Handoff | Topic |
| --- | --- |
| 12 | Slot-correct replay and snapshots (in flight) |
| 13 | This contract |
| 14 | Triangle and multi-pair replay |
| 15 | Read-only Research API (serves v1 and v2) |
| 16 | MEVana-UI research page |

## Execution model

Implement this directly in this repository with an interactive coding-assistant session (Codex or Claude Code), following the OpenSpec workflow.
- Do **not** use the AgentEngineering factory.
- Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/request-v2-20261030`, on branch `feat/request-contract-v2-20261030`, from `origin/main`.
- Merging is the operator's decision. If handoff 12 merges first, rebase onto it before finishing.

## Entry state (verified 2026-10-30)

These are in `dotnet/src/MEVana.Research.Core/`:

| File | What it does |
| --- | --- |
| `ExperimentRequest.cs` | Raw immutable DTO with 8 fields. Nulls, order and duplicates are preserved. |
| `ExperimentRequestSyntaxValidator.cs` | Identifiers are 1–64 `[A-Za-z0-9_-]`; `tokenPair` is exactly 2; `poolIds` is 2–16 distinct; the window is ≤ 24 h; trade bounds are positive and ordered. |
| `ExperimentRequestValidator.cs` | Catalog checks: dataset, availability, strategy, coverage, tokens. Every pool must contain both pair tokens. Bound = `min(dataset max, tokenPair[1] max)`. |
| `ExperimentRequestCanonicalizer.cs` | Canonical JSON with `canonicalizationVersion: 1`. `poolIds` are sorted ordinally. |
| `ExperimentResultValidator.cs` | `RequestFingerprint.FromRequest(ExperimentRequest)` takes a SHA-256 of the canonical form. The 3-argument `Validate` checks the fingerprint, version, dataset, strategy and window. |

The OpenSpec baseline is `openspec/specs/research-experiment-request/spec.md`.

## Design

**1. New type.** Add `ExperimentRequestV2` as a separate class. Do not subclass or change `ExperimentRequest`. It is a raw immutable DTO with the same preservation rules as v1, with these fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `datasetId` | id | Same rule as v1 |
| `strategyVersion` | id | Same rule as v1 |
| `windowStartUtc` / `windowEndUtc` | instant | Same rule as v1 |
| `quoteTokenId` | id | The asset every route starts and ends in. All sizes and profits are in its atoms. |
| `tokenIds` | 2–8 distinct ids | The token universe. Must include `quoteTokenId`. |
| `poolIds` | 2–32 distinct ids | The **route-leg** universe: exactly the pools evaluated as legs. |
| `routeShapes` | 1–2 distinct values from `two_leg`, `triangle` | Which route shapes are evaluated |
| `priceInputPoolIds` | 0–4 distinct ids | Cost-model price inputs only. They may overlap `poolIds`. |
| `minTradeSizeQuoteAtoms` / `maxTradeSizeQuoteAtoms` | long | Same rule as v1 |

**2. Syntax validation.** Add `ExperimentRequestV2SyntaxValidator`.
- Use v1's identifier, window and trade rules and codes.
- Add these codes:
  - `invalid_token_ids`;
  - `invalid_quote_token`, when the quote token is missing from `tokenIds`;
  - `invalid_pool_ids`, using v2 bounds;
  - `invalid_route_shapes`;
  - `invalid_price_input_pools`.
- Results are deterministic, read-only `ValidationIssue`s. A null request returns `request/required`.

**3. Catalog validation.** Add `ExperimentRequestV2Validator`.
- Apply v1's dataset, availability, strategy and coverage checks.
- **Tokens:** every token in `tokenIds` must be in the dataset (`incompatible_token`).
- **Pools:** every pool in `poolIds` and `priceInputPoolIds` must be in the dataset (`incompatible_pool`). Each pool in `poolIds` must have **both** of its tokens in `tokenIds` (`incompatible_pool`).
- **Scope truthfulness:** every pool in `poolIds` must be usable by at least one route of a requested shape (`unused_pool`).
  - A `two_leg` route uses two distinct pools on the same pair, where the pair includes `quoteTokenId`.
  - A `triangle` uses three distinct pools forming the cycle quote → X → Y → quote, over three distinct tokens.
- **Shape possibility:** each requested shape must yield at least one route (`shape_unsatisfiable`).
- **Trade bound:** `min(dataset max, quote token max)` (`invalid_trade_size`).
- Route enumeration helpers must be deterministic and testable. Expose a pure function that returns the ordered route list (pool sequence plus token path) for a valid request. The later replay must use this same definition.

**4. Canonicalization.** Add `ExperimentRequestV2Canonicalizer`.
- Output is canonical JSON with `canonicalizationVersion: 2`, in this field order:

  `canonicalizationVersion`, `datasetId`, `strategyVersion`, `windowStartUtc`, `windowEndUtc`, `quoteTokenId`, `tokenIds`, `poolIds`, `routeShapes`, `priceInputPoolIds`, `minTradeSizeQuoteAtoms`, `maxTradeSizeQuoteAtoms`.
- `tokenIds`, `poolIds`, `routeShapes` and `priceInputPoolIds` are sorted ordinally.
- Times use the v1 `"O"` UTC format.
- It throws on syntactically invalid input, as v1 does.
- Add `RequestFingerprint.FromRequest(ExperimentRequestV2)` as a new overload. The v1 method stays unchanged.

**5. Result linkage.** Add a new overload, `ExperimentResultValidator.Validate(ExperimentResult, ResearchCatalog, ExperimentRequestV2)`.
- It applies the same checks as the v1 overload, plus `requestCanonicalizationVersion == 2`.
- `ExperimentResult` itself does not change. Existing v1 results and validation stay identical.
- A v1 fingerprint must never validate against a v2 request, or the reverse (`request_mismatch`).

**6. JSON.** If `ResearchContractJson` parses v1 requests, add the matching v2 parsing.
- A document that carries `canonicalizationVersion: 2` must not be accepted by the v1 path.
- A document that carries version 1 must not be accepted by the v2 path.

**7. OpenSpec.**
- New capability spec: `research-experiment-request-v2`.
- The change also adds a requirement to `research-experiment-request` (v1) stating that v1 is frozen and unaffected by v2.
- Scenarios must cover every validation code above, canonical determinism, the fingerprint version separation, and the route-enumeration examples.

## Tests (xUnit, new files only)

- **Mirror v1:** for each v1 syntax, catalog and canonicalization test, add a v2 equivalent.
- **Golden canonical bytes and SHA-256** for the three requests below. Use the handoff 09 allowlist's token and pool ids from the existing test catalog style.
  - **R1, triangle only:** tokens [SOL, USDC, USDT], quote USDC, all 13 standard-fee pools.

    Expected route count:
    - one SOL/USDC, one USDC/USDT and one SOL/USDT pool per cycle;
    - both directions;
    - 6 SOL/USDC × 2 USDC/USDT × 5 SOL/USDT pools × 2 directions = **120** triangles.

    Assert the exact count from the enumerator.
  - **R2, `two_leg` plus `triangle`, quote USDC.** Two-leg routes exist only on pairs that contain USDC: SOL/USDC (30) and USDC/USDT (2). Total = 30 + 2 + 120 = **152**.
  - **R3, `two_leg` only:** tokens [SOL, USDC, USDT], quote USDC, with a SOL/USDT pool included. Assert `unused_pool` for that pool.
- **v1 regression:** all existing tests stay unchanged and pass. Assert the handoff 11 v1 fixture fingerprints are unchanged. Read them, but don't edit them.
- **Fingerprint separation:** a v1 and a v2 request with equivalent scope have different fingerprints, and each fails against the other's result.

## Constraints

- **.NET only:** new files in `MEVana.Research.Core` and new test files, plus OpenSpec. The only edit allowed to an existing file is adding an overload where unavoidable (`RequestFingerprint`, `ExperimentResultValidator`).
- **No behavior change to v1** in any way.
- **No Rust changes.** No network, no credentials, no `.env`.
- **Do not touch** the files handoff 12 owns (listed in Purpose).
- Date every document. Commit and push all work; lose nothing.

## Validation

- `dotnet build` and `dotnet test dotnet/MEVana.slnx`: all prior tests plus the new ones;
- `openspec validate --all --strict`;
- `./scripts/ci/repo_hygiene.sh`;
- `git diff --check`;
- a non-printing credential scan over the diff.

Rust checks are not required, because no Rust changes.

## Closeout and exit

Write a dated closeout doc covering:
- the fields and codes added;
- the route-enumeration definition with counts for R1–R3;
- the golden hashes;
- confirmation that v1 is unchanged.

Archive the OpenSpec change. Push the branch clean and synchronized.

If a requirement here cannot be met without changing v1 behavior, stop and report instead.
