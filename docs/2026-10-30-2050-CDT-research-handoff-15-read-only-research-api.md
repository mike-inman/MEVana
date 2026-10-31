# 2026-10-30 20:50 CDT — Handoff 15: Read-only Research API

## Purpose

Handoffs 11–14 produced validated research results, but they exist only as JSON fixtures and local replay directories. This slice builds a small, read-only ASP.NET Core API that serves them, so the MEVana-UI research page (handoff 16) has a truthful data source.

The main risk is not the HTTP code. It is presenting weak evidence as stronger than it is. This API must keep these visible on every experiment:
- the evidence kind;
- which quote asset the profit is in;
- how much of the candidate set could actually be evaluated;
- that "not measured" is different from zero.

This slice is:
- .NET plus OpenSpec, with one line added to `config/ports.toml`;
- **loopback-only and local;**
- no database, Docker, deployment, authentication, CORS or nginx;
- no Rust changes, no replay, no provider calls, no capture, no credentials and no `.env`;
- no MEVana-UI changes.

## Execution model

Implement this directly in this repository with an interactive coding-assistant session, following the OpenSpec workflow.
- Do **not** use the AgentEngineering factory.
- Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/research-api-20261030`, on branch `feat/research-api-20261030`, from `origin/main`.
- First confirm that `origin/main` contains `aca37c12`. If it does not, stop and report.
- Set the branch's upstream to `origin/feat/research-api-20261030`, not `origin/main`.
- Merging is the operator's decision.

**Permitted network use:** `git fetch`, and pushing this one branch. Nothing else. Do not add NuGet packages.

## Entry state (verified 2026-10-30)

| Area | State |
| --- | --- |
| `dotnet/src/MEVana.Research.Api/Program.cs` | Three-line empty host, already in `MEVana.slnx`. References Core and Persistence. |
| `dotnet/src/MEVana.Research.Persistence` | Project exists with no source files. References Core. |
| Catalog | The only capture-backed definition is `CaptureReplayCatalog.cs`, in the **test** project. |
| JSON | `ResearchContractJson` only writes. Request and result parsing exists only in tests. |
| Test project | References Core only. Fixtures are copied to the test output. |
| Port 8098 | Not listed in `config/ports.toml`; nothing was listening when checked. |

## Design

### 1. What is served

Serve exactly four experiments, all on dataset `orca-wp-snapshot-20261030`:

| `resultId` | Contract | Scope | Quote asset |
| --- | --- | --- | --- |
| `replay-v2-463353643-a-confirmed-l1-tip10000` | v1 | SOL/USDC two-leg | USDC |
| `replay-v2-463353643-b-confirmed-l1-tip10000` | v1 | SOL/USDT two-leg | USDT |
| `replay-v2-463353643-c-confirmed-l1-tip10000` | v1 | USDC/USDT two-leg | USDT |
| `replay-v2-463353643-usdc-confirmed-l1-tip10000` | v2 | 152-route USDC universe | USDC |

**Deliberately not served:**
- handoff 11 results, because handoff 12 superseded them after correcting slot semantics;
- synthetic results;
- ranking artifacts and execution outcomes, because only synthetic examples exist. Handoffs 07 and 08 proposed serving them from fixtures, but with trading disabled, a "paper landed" outcome in a research API could easily be misread. They wait until real artifacts exist.

### 2. Seed data

Create `dotnet/data/research-artifacts/` holding:
- `catalog.json`: the snapshot dataset;
- `manifest.json`: the four experiments, listing file names, a source closeout path and a `limitations` string array;
- byte-identical copies of the four request and result fixture pairs;
- the handoff 14 `summary.json`, copied from `~/MEVanaData/replay/handoff14-final-v2-default1/summary.json`. Its SHA-256 must begin `8edff656b82130ee`.

Tests must assert:
- every copied request and result equals its `Fixtures/` file byte for byte;
- the loaded catalog matches `CaptureReplayCatalog.CreateSnapshot()`.

Do not move or edit existing fixtures, and do not edit `CaptureReplayCatalog.cs`.

### 3. Persistence: a validated, read-only store

Add source to `MEVana.Research.Persistence`:
- strict parsers for the catalog, v1 and v2 requests, and results. Reject unknown properties, missing properties and duplicates;
- a v1 request document must carry `canonicalizationVersion: 1`, and a v2 document must carry 2. Neither parser accepts the other's document. This is the rule from handoff 13 section 6, which did not apply until now;
- `ResearchArtifactStore`, which loads everything once at startup into immutable in-memory structures.

The artifact directory is a configured path. Request handlers read only from the in-memory store, and no value from a URL may reach the filesystem.

**Fail closed.** If any check below fails, startup fails and the API does not serve anything:
1. Parsing succeeds.
2. Re-canonicalizing each request and result reproduces the file's exact bytes, using the matching v1 or v2 canonicalizer.
3. The request's SHA-256 equals the result's `requestSha256`.
4. Request validation and the 3-argument `ExperimentResultValidator.Validate` overload return no issues.
5. `evidenceKind` is `Simulated`. `realizedNetProfitQuoteAtoms` is null for every policy.
6. Every numeric value in an artifact is within ±(2^53 − 1), so a JavaScript client cannot silently lose precision.
7. For the v2 experiment, per-shape counts in `summary.json` sum to the result's policy totals.

### 4. HTTP surface

GET and HEAD only. The path's `v1` is the **API** version, and has nothing to do with contract versions 1 and 2.

| Path | Response |
| --- | --- |
| `GET /health` | `{"status":"ok"}`. No data. |
| `GET /research/v1/datasets` | Dataset list |
| `GET /research/v1/datasets/{datasetId}` | One dataset |
| `GET /research/v1/experiments` | Experiment summaries, ordered by `resultId` (ordinal). No pagination. |
| `GET /research/v1/experiments/{resultId}` | One full experiment |

Any other method must return 405, and an unknown path or id must return 404 as `{"code":"not_found"}`. A malformed id must return 404 too, and must not be echoed back in the response.

**Each experiment summary contains:**
- `resultId`, `datasetId`, `strategyVersion`, `engineVersion`, `costModelVersion`, `requestCanonicalizationVersion`, `requestSha256`, `windowStartUtc` and `windowEndUtc`;
- `evidenceKind` and `status`;
- `quoteTokenId` and `quoteTokenDecimals`. Use `tokenPair[1]` for v1 and `quoteTokenId` for v2;
- `scope`: `routeShapes`, token ids and pool count. For v1, `routeShapes` is `["two_leg"]`;
- `policies`: the three policy results, with `missedCandidates` and `realizedNetProfitQuoteAtoms` preserved as null;
- `coverage`, described below.

**The full experiment adds:**
- `request` and `result`, the stored JSON unchanged, so a client can verify the hash;
- `routeCounts`: from `ExperimentRequestV2RouteEnumerator` for v2, and null for v1;
- `shapes` and `poolDirections`: from `summary.json` for v2, and null for v1;
- `limitations` and `sourceCloseout`.

Handoff 14's pool-direction rows are diagnostic probes at raw input amounts, not chained route amounts. The response must label them `diagnosticProbe: true`.

### 5. Coverage and aggregation rules

`coverage` holds one entry per policy:

| Field | Definition |
| --- | --- |
| `candidatesEvaluated` | From the result |
| `unevaluableCandidates` | Sum of rejections with reason `state_gap`, `insufficient_state` or `tick_array_absent` |
| `evaluableCandidates` | `candidatesEvaluated − unevaluableCandidates` |

Use integer counts only. The client calculates percentages.

Keep one entry per policy. On the handoff 14 result the totals happen to match, but the reason mix differs, because FixedUniverse looks only at the smallest size. Do not assume the totals will always match.

**No cross-experiment totals.** Experiments use different quote assets, and pair and universe results overlap. No endpoint may sum or rank profit across experiments.

Responses must not contain any field that describes an experiment as profitable, successful or recommended. Only measured values are returned.

### 6. Hosting

- Default bind: `http://127.0.0.1:8098`. Add `research_api = 8098` to `config/ports.toml`.
- Allow the port and artifact path to be configured, but startup must **fail on any non-loopback bind address**.
- No CORS. The handoff 16 UI will call this from Next.js server code, so the browser never needs direct access.
- Without authentication, the loopback restriction is this slice's only access control. Keep that explicit in the OpenSpec spec, along with the rule that exposing the API beyond loopback needs a later handoff with real authentication.
- No request logging of anything beyond method, path template and status.

### 7. Structure for testing

Move app construction into a function that a test can call, and keep `Program.cs` as a thin entry point. Tests start the real app on `127.0.0.1` with port 0 and use `HttpClient`.

Do **not** add `Microsoft.AspNetCore.Mvc.Testing` or any other package. Add a `FrameworkReference` to `Microsoft.AspNetCore.App` and project references to the Api and Persistence projects in the test project. These are the only edits allowed to its `.csproj`.

## Tests (xUnit, new files only)

- **Startup failure:** one test per fail-closed check, using temporary tampered copies: a changed byte, a wrong hash, a v2 document in the v1 parser and the reverse, an unknown property, a `Live` evidence kind, an unsafe integer, and a summary that doesn't sum.
- **Seed integrity:** the byte-equality and catalog assertions from section 2.
- **Endpoints:** status codes and bodies for every route, 405 on POST, PUT, PATCH and DELETE, 404 on unknown and malformed ids, and no echoing of the id.
- **Evidence truth:** `missedCandidates` and realized profit are JSON null, not 0. USDT experiments report USDT. Every summary has `evidenceKind` and `coverage`.
- **Coverage values** for the v2 experiment, per policy:

  | Policy | Evaluated | Unevaluable | Evaluable |
  | --- | ---: | ---: | ---: |
  | FixedUniverse | 339,720 | 81,156 | 258,564 |
  | SimpleRules | 339,720 | 81,156 | 258,564 |
  | ModelRanking | 339,720 | 81,156 | 258,564 |

  These come from the handoff 14 `summary.json`. If the served values differ, stop and report it. Do not change the test to fit.
- **Hash verification:** a test client recomputes SHA-256 over the served `request` and matches `requestSha256`. For this to hold, the API must serve those bytes exactly, for example as a raw JSON value.
- **Determinism:** two starts return byte-identical responses. Commit golden response files for the experiment list and for the v2 experiment.
- **Bind guard:** `0.0.0.0` and any public address fail startup.
- **Isolation:** no test uses a fixed port. All existing tests pass unedited.

## Smoke test

After the tests pass, start the built API on its default loopback address and call `/health`, the list, and the v2 detail route with `curl`. Then stop it and confirm that nothing is listening on 8098. Record the output in the closeout.

## Constraints

- **No v1 or v2 contract behavior change.** Do not edit any `MEVana.Research.Core` file, or any existing fixture or test file. If a Core change looks necessary, stop and report.
- Do not change `crates/`, `MEVana-UI`, Docker files or anything under `~/MEVanaData`.
- No database, background job, write endpoint, file upload or dynamic artifact discovery.
- Leave no process running.
- Date every document. Commit and push all work; lose nothing.

## Validation

- `dotnet build` and `dotnet test dotnet/MEVana.slnx`: all prior tests plus the new ones;
- `openspec validate --all --strict`;
- `./scripts/ci/repo_hygiene.sh`;
- `git diff --check`;
- a non-printing credential scan over the diff;
- a check that `crates/` and `MEVana.Research.Core` have no diff.

Rust checks are not required, because no Rust changes.

## Closeout

Write a dated closeout doc covering:
- the endpoints and response shapes;
- the fail-closed checks and how each was tested;
- coverage values for all four experiments;
- the smoke test output;
- what is not served and why;
- limitations: local loopback only, no authentication, seed data is static, and all results are about eight minutes of dev/test Processed data with no fill or inclusion modeling;
- confirmation that Core, the contracts and existing fixtures are unchanged.

Archive the OpenSpec change. Push the branch clean and synchronized.

## Exit criteria

The slice either stopped with a precise report, **or** all of the following are true:
- the API serves exactly the four experiments from a validated in-memory store;
- it refuses to start on any integrity failure or non-loopback bind;
- it exposes no write path;
- evidence kind, quote asset, coverage and null-versus-zero are tested;
- everything above is documented, archived and pushed, and nothing is left running.

If a requirement here cannot be met without changing Core or an existing contract, stop and report instead.

## Later slices

- Handoff 16: MEVana-UI research page. It adds a `research` service endpoint and calls this API from server code, behind the existing Clerk sign-in.
- Authentication and deployment of this API, if it ever leaves loopback.
- A longer, stronger-commitment capture on a server host, with better SOL/USDT tick-array coverage.
- Serving ranking artifacts and execution outcomes, once real ones exist.
