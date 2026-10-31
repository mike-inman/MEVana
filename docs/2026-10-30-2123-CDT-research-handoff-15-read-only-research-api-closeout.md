# 2026-10-30 21:23 CDT — Handoff 15 closeout: read-only Research API

Created: 2026-10-30 21:23 CDT.

## Outcome

`MEVana.Research.Api` now serves one dataset and exactly four `Simulated` experiments, read-only, on `http://127.0.0.1:8098`. Everything it serves is validated at startup into an in-memory store. Any integrity failure or non-loopback address stops startup. This slice adds no database, deployment, authentication, CORS or nginx, and it does not change Core, the contracts, existing fixtures or tests, Rust, or MEVana-UI.

This session implemented the handoff directly in Claude Code rather than in Codex. The branch `feat/research-api-20261030` is for operator review; merging is the operator's decision.

## Endpoints

GET and HEAD only. The path's `v1` is the API version, not a contract version.

| Path | Response |
| --- | --- |
| `/health` | `{"status":"ok"}` |
| `/research/v1/datasets` | `{"datasets":[…]}` |
| `/research/v1/datasets/{datasetId}` | One dataset |
| `/research/v1/experiments` | `{"experiments":[…]}`, ordered ordinally by `resultId`, no pagination |
| `/research/v1/experiments/{resultId}` | One full experiment |

Every other method returns 405 with `Allow: GET, HEAD`, on any path. An unknown path or id returns 404 with `{"code":"not_found"}`, and the id is not echoed. Request logs hold only the method, path template and status; non-read methods are logged as `OTHER`.

A summary contains the identifiers and versions, window, `evidenceKind`, `status`, `quoteTokenId`, `quoteTokenDecimals`, `scope`, `policies` and `coverage`. The detail adds the unchanged `request` and `result`, `routeCounts`, `shapes`, `poolDirections`, `limitations` and `sourceCloseout`. The last v2-only fields are null for v1. Pool-direction rows are served as `{diagnosticProbe:true, probeInputAtoms, rows}`, because they are raw-amount probes and not chained route amounts.

## Fail-closed checks

| Check | Failure code | Tested by |
| --- | --- | --- |
| Parsing | `invalid_json`, `unknown_property`, `missing_property`, `duplicate_property`, `invalid_value`, `missing_file`, `invalid_file_name` | Truncated file, added and duplicated property, deleted file, path-like manifest names |
| Canonical bytes | `noncanonical_bytes` | Added whitespace in a request (with its hash relinked) and in a result |
| Request hash | `request_hash_mismatch` | Changed request byte, changed hash |
| Contract validation | `contract_validation_failed` | Catalog altered so the strategy is unsupported |
| Evidence | `unsupported_evidence` | `Live` evidence kind, non-null realized profit |
| Safe integers | `unsafe_integer` | ±2^53 and a non-integer |
| Summary reconciliation | `summary_mismatch` | Changed shape count, summary on a v1 experiment |
| Contract separation | `contract_version_mismatch` | Each parser given the other's document, manifest version crossed |

Handoff 13 section 6 now applies through these parsers. An end-to-end test confirms that a failed artifact prevents `StartAsync` from returning a server.

Seven safeguards were also broken deliberately, one at a time: the hash, canonical, evidence, loopback, duplicate and coverage-reason logic, plus the fallback route. The suite failed each time, and passed 294/294 once they were restored.

## Coverage served

Totals match across all three policies in each experiment; only the reason mix differs. The client calculates percentages.

| Experiment | Quote | Evaluated | Unevaluable | Evaluable | Evaluable share |
| --- | --- | ---: | ---: | ---: | ---: |
| A SOL/USDC | USDC | 67,050 | 180 | 66,870 | 99.73% |
| B SOL/USDT | USDT | 44,700 | 24,639 | 20,061 | 44.88% |
| C USDC/USDT | USDT | 4,470 | 12 | 4,458 | 99.73% |
| 152-route universe | USDC | 339,720 | 81,156 | 258,564 | 76.11% |

The 76.11% covers the whole universe. Handoff 14's 69.81% is the triangle share alone, and is available in `shapes`.

## Smoke test

Run from the final build on the default address, then stopped:

```text
Url=http://0.0.0.0:8098 -> ArgumentException: Research API may bind only to a literal loopback http address. (exit 134)
listen: dotnet 127.0.0.1:8098
GET /health -> {"status":"ok"} [200]
GET /research/v1/experiments ->
  replay-v2-463353643-a-confirmed-l1-tip10000 Simulated 66870 / 67050
  replay-v2-463353643-b-confirmed-l1-tip10000 Simulated 20061 / 44700
  replay-v2-463353643-c-confirmed-l1-tip10000 Simulated 4458 / 4470
  replay-v2-463353643-usdc-confirmed-l1-tip10000 Simulated 258564 / 339720
GET .../replay-v2-463353643-usdc-confirmed-l1-tip10000 -> {triangle: 120, two_leg: 32, total: 152} 63788e9af7498553 diagnosticProbe=true
POST /research/v1/experiments -> {"code":"method_not_allowed"} [405]
listening after stop: 0
```

Run it with `dotnet MEVana.Research.Api.dll` from the build output. `dotnet run` was not tested. The native `MEVana.Research.Api` launcher fails on this machine because it looks in `/usr/local/share/dotnet`, which has no .NET 10 runtime. Port and artifact directory are set with `MEVANA_RESEARCH_Url` and `MEVANA_RESEARCH_ArtifactDirectory`, or `--Url` and `--ArtifactDirectory`.

## Not served, and why

- Handoff 11 results: superseded by the handoff 12 slot correction.
- Synthetic results: not evidence.
- Ranking artifacts and execution outcomes: only synthetic examples exist, and a "paper landed" outcome could be misread while trading is disabled. This narrows the older plan in handoffs 07 and 08.

## Validation and limits

- `dotnet build dotnet/MEVana.slnx`: zero warnings and errors. `dotnet test dotnet/MEVana.slnx`: 294 passed, none failed or skipped (217 prior, 77 new).
- `openspec validate --all --strict`, `./scripts/ci/repo_hygiene.sh` and `git diff --check`: passed. Hygiene reports only its existing timestamp and script-location warnings.
- A non-printing credential scan over the diff found nothing.
- No diff in `crates/` or `dotnet/src/MEVana.Research.Core`, and no existing fixture or test file changed. The only edits to existing files are `Program.cs`, the Api and test `.csproj` files, and one line in `config/ports.toml`. No package was added.

The API is local only and has no authentication, so loopback is its only access control. The seed is static, and a new result needs a seed change and restart. Every served result rests on about eight minutes of dev/test Processed data from a laptop, with no fill, inclusion, slippage or realized-profit modeling. `ModelRanking` is the labeled `ranking-v0-heuristic`; no learned model was involved. Coverage is unequal across experiments, and B remains thin.

The OpenSpec change `add-research-api` is archived with its spec synced.
