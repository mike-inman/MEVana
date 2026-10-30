# 2026-10-30 13:25 CDT — Handoff 13 closeout: ExperimentRequestV2 contract

Created: 2026-10-30 13:25 CDT

## Outcome

A separate `ExperimentRequestV2` now names a multi-token route universe, quote token, evaluated route-leg pools, requested shapes, cost-model price-input pools, and integer quote-atom trade bounds. It is a raw immutable DTO: invalid values and offsets remain inspectable, while submitted lists are defensive read-only copies. The original `ExperimentRequest` and every v1 method body remain unchanged. This is contract work only; no replay, provider call, triangle evaluation, trading, Rust, `.env`, or handoff 12-owned test file was changed.

The [13:19 CDT amendment](2026-10-30-1319-CDT-research-handoff-13-amendment-v1-cross-version-rejection.md) resolved the earlier [blocker](2026-10-30-1249-CDT-research-handoff-13-v1-result-linkage-blocker.md). A version-1 result checked against a v2 request returns `request/request_mismatch` from the new overload. A version-2 result checked against a v1 request retains the existing `requestCanonicalizationVersion/unsupported_canonicalization_version` rejection. Both new v2 overloads retain an optional parameter so existing literal-null calls bind to the v1 overloads and compile without editing v1 tests.

`ResearchContractJson` is a writer-only helper and does not parse v1 requests, so the conditional JSON parsing requirement did not apply.

## Fields, validation and routes

V2 adds `quoteTokenId`, `tokenIds` (2–8), `routeShapes` (1–2 of `two_leg` and `triangle`) and `priceInputPoolIds` (0–4). Route-leg `poolIds` allow 2–32. Dataset, strategy, window and long trade bounds follow the v1 rules. Syntax adds `invalid_token_ids`, `invalid_quote_token`, `invalid_pool_ids` with the v2 bound, `invalid_route_shapes` and `invalid_price_input_pools`. Catalog validation adds `incompatible_token`, `incompatible_pool` for either pool list or a route leg outside the token universe, `shape_unsatisfiable` and `unused_pool`; trade size is bounded by the smaller dataset and **quote-token** limit. Null and shared errors remain deterministic read-only `ValidationIssue`s.

The pure route enumerator sorts by shape, pool sequence and token path ordinally. A `two_leg` route uses two distinct same-pair pools containing the quote token and returns quote → other → quote. A `triangle` uses three distinct pools and tokens in quote → X → Y → quote order. Price-input pools do not become route legs; they may overlap route-leg pools.

| Request | Scope and shape | Enumerated routes | Validation |
| --- | --- | ---: | --- |
| R1 | 6 SOL/USDC, 5 SOL/USDT, 2 USDC/USDT; quote USDC; triangle | 120 = 6 × 5 × 2 × 2 directions | Valid |
| R2 | Same 13 pools; quote USDC; two_leg + triangle | 152 = 30 + 2 + 120 | Valid |
| R3 | Six SOL/USDC plus one SOL/USDT; quote USDC; two_leg | 30 possible from SOL/USDC | `unused_pool` for the SOL/USDT leg pool |

The later replay must use this enumerator's definition. These tests describe route shapes; they do not demonstrate executable opportunities or profit.

## Canonical identity and tests

Version-2 canonical JSON uses the specified fixed field order, invariant UTC `O` timestamps and ordinally sorted token, route-pool, shape and price-input lists. Golden fixture bytes contain no trailing newline. The SHA-256 fingerprints of their canonical UTF-8 bytes are:

| Golden request | SHA-256 |
| --- | --- |
| R1 | `e788d5b9ed37d5dc122c34f3eeb0e2069193fa44ba72e5aa1c3d71f14a03498b` |
| R2 | `bf41b52a0a6bd29392025c2033e058f75a6d366e3b3bbc893cb415310e43c844` |
| R3 | `ff9f467fd1e27be2d98b3eed1168064d81e06e3091a3ca26484839c430c64f74` |

Tests assert byte-for-byte fixture equality, those independent fixed hashes, order, culture and offset invariance, invalid syntax, catalog bounds, route counts and unused-pool rejection. They also read the unchanged handoff 11 request/result fixtures and verify all three v1 canonical byte sequences and fingerprints. An equivalent two-leg v1 and v2 scope has distinct fingerprints. New result linkage checks the v2 fingerprint, dataset, strategy, window and version without widening the v1-only result syntax path.

Validation on the local .NET toolchain passed: `dotnet build dotnet/MEVana.slnx --no-restore` with zero warnings/errors, `dotnet test dotnet/MEVana.slnx --no-restore` **213/213**, `openspec validate --all --strict`, repository hygiene, `git diff --check`, and a non-printing secret scan over the branch diff. Hygiene reported only its pre-existing repository timestamp/script-placement warnings. The three new golden files are synthetic contract examples built with public allowlist IDs; they are not market-evidence results.

The OpenSpec change `experiment-request-v2` is synced and archived with this closeout. Branch `feat/request-contract-v2-20261030` is for operator review; merging to `main` is the operator's decision.
