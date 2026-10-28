# 2026-10-28 08:23 CDT — Research Stage 0 handoff 08 closeout

## Outcome and Stage 0 complete

The **Stage 0 contract set is complete**. All five capabilities have main specs:

| Contract | Main specification |
| --- | --- |
| Dataset catalog | `openspec/specs/research-dataset-catalog/spec.md` |
| Experiment request | `openspec/specs/research-experiment-request/spec.md` |
| Experiment result | `openspec/specs/research-experiment-result/spec.md` |
| Ranking artifact / advisory WATCH | `openspec/specs/research-ranking-artifact/spec.md` |
| Execution outcome | `openspec/specs/research-execution-outcome/spec.md` |

Ranking and outcome contracts are implemented, tested and archived. The ranking archive contains seven requirements; the outcome archive contains eight. Each merged main spec was checked against the delta's purpose and every full requirement/scenario block, with no delta-operation headers left in the main spec.

Final solution tests: **Passed 194, Failed 0, Skipped 0, Total 194**, versus the verified baseline of **79**: **115 additional passing tests** (20 ranking tests and 95 outcome tests, including the 54-case mode/evidence/state matrix). The final solution build reports **0 warnings and 0 errors**. Strict validation passes **all five main specs** with no active changes.

This completes the contract slice only. Contract tests establish neither ranking effectiveness nor actual execution/landing/reconciliation. Rust finding F3 remains a separate implementation repair; the hotpath has not been changed.

## Delivered implementation

- One raw immutable `ResearchRoute`, shared by both contracts; syntax checks ordered closed cycles and catalog semantics check route membership/adjacency and first-token quote bounds.
- Raw ranking/entry DTOs, syntax and catalog/time validators, explicit version-1 canonicalization and a fixed advisory WATCH kind.
- Raw outcome DTO, execution mode and terminal-state enums, syntax/catalog validation, independent nullable profit and unsigned cost fields, explicit version-1 canonicalization and a pure idempotency conflict helper.
- Public DTO and canonical JSON field allowlist tests, including nested shapes, verify that the contracts expose no trading-authority or floating-point fields. Transaction signatures are syntax-checked submission identifiers only.
- Three independently authored Python standard-library JSON golden fixtures, compared byte-for-byte with C# output:
  - `dotnet/tests/MEVana.Research.Tests/Fixtures/2026-10-28-synthetic-ranking-artifact-v1.json`;
  - `dotnet/tests/MEVana.Research.Tests/Fixtures/2026-10-28-synthetic-rejected-outcome-v1.json`;
  - `dotnet/tests/MEVana.Research.Tests/Fixtures/2026-10-28-synthetic-paper-landed-outcome-v1.json`.

All fixtures explicitly say **Synthetic**, use synthetic identifiers and contain no market evidence. The paper Landed fixture records a synthetic paper terminal state, never an actual live landing or realized profit.

## Commands and exact results

All commands ran from `/Users/michaelinman/Repos/MEVana`. `openspec` below denotes `~/.local/bin/openspec`. Every validation/build/test/archive command in this run exited 0; no failed validation or test attempts occurred.

| Command or check | Exact result |
| --- | --- |
| Entry `git status --short`, `git branch --show-current`, `git log -3 --oneline` | Clean working tree; `main`; entry HEAD `e4ae5f48`, preceded by `7ab2821d` and `e025c988`. |
| `openspec context --json` | Resolved root `/Users/michaelinman/Repos/MEVana`, nearest OpenSpec root. |
| Entry `openspec list --json` | `changes: []`. |
| Entry `dotnet test dotnet/MEVana.slnx --verbosity quiet` | Passed 79, Failed 0, Skipped 0, Total 79. |
| `openspec validate --all --strict` after project context update | 3 passed, 0 failed (three existing specs). Context was committed separately as `bcc45a86`. |
| `openspec list --specs` | Initially catalog 3 requirements, request 5 and result 8; subsequently also ranking 7. |
| `openspec show research-experiment-result --type spec --json --no-scenarios` and corresponding ranking command | Existing capability overviews read successfully; full relevant specs were also read. |
| `openspec new change add-research-ranking-artifact` | Created `spec-driven` change. |
| `openspec instructions proposal/specs/design/tasks --change add-research-ranking-artifact --json` (each artifact separately), plus `openspec status --change add-research-ranking-artifact --json` | Required proposal, design, tasks and spec paths resolved inside this repo. Artifacts completed in dependency order; no extra artifact rules. |
| `openspec validate --all --strict` for ranking proposal | 4 passed, 0 failed (three specs and one change). |
| `openspec instructions apply --change add-research-ranking-artifact --json` | Progress advanced 0/2 → 1/2 → 2/2; final state `all_done`. Context files were read before implementation. |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet --filter FullyQualifiedName~RankingArtifactContractTests` | Passed 20, Failed 0, Skipped 0, Total 20. |
| `dotnet build dotnet/MEVana.slnx` before ranking archive | Build succeeded, 0 warnings, 0 errors. |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet` before ranking archive | Passed 99, Failed 0, Skipped 0, Total 99. |
| `openspec validate --all --strict` before ranking archive | 4 passed, 0 failed. |
| `openspec instructions archive --change add-research-ranking-artifact --json`, status and list commands | Complete artifact graph, completedTasks 2, totalTasks 2; no extra archive guidance. |
| `openspec archive add-research-ranking-artifact --yes` | Complete; created ranking main spec, +7 requirements, 0 modified/removed/renamed; archive `2026-10-28-add-research-ranking-artifact`. |
| Python ranking archive-content assertions | Purpose and all 7 full requirement blocks match delta; canonical main-spec structure valid; main spec dated. |
| `openspec validate --all --strict` after ranking archive | 4 passed, 0 failed (four main specs). |
| `openspec new change add-research-execution-outcome` | Created `spec-driven` change after ranking was archived. |
| `openspec instructions proposal/specs/design/tasks --change add-research-execution-outcome --json` (each artifact separately), plus status commands | Repo-local artifact paths and dependencies resolved; all four artifacts completed; standing project context applied. |
| `openspec validate --all --strict` for outcome proposal | 5 passed, 0 failed (four specs and one change). |
| `openspec instructions apply --change add-research-execution-outcome --json` | Progress advanced 0/2 → 1/2 → 2/2; final state `all_done`. |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet --filter FullyQualifiedName~ExecutionOutcomeContractTests` | Passed 95, Failed 0, Skipped 0, Total 95. |
| Python byte comparison of all baseline `.cs` files listed by `git ls-tree -r --name-only 7ab2821d dotnet/src` | All **12** existing source files equal `git show 7ab2821d:<path>` byte-for-byte, including all five catalog/request source files and every result source file. |
| `dotnet build dotnet/MEVana.slnx` before outcome archive | Build succeeded, 0 warnings, 0 errors. |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet` before outcome archive | Passed 194, Failed 0, Skipped 0, Total 194. |
| `openspec validate --all --strict` before outcome archive | 5 passed, 0 failed. |
| `openspec instructions archive --change add-research-execution-outcome --json`, status and list commands | Complete artifact graph, completedTasks 2, totalTasks 2; no extra archive guidance. |
| `openspec archive add-research-execution-outcome --yes` | Complete; created outcome main spec, +8 requirements, 0 modified/removed/renamed; archive `2026-10-28-add-research-execution-outcome`. |
| Python outcome archive-content assertions | Purpose and all 8 full requirement blocks match delta; no delta-operation headings; main spec dated. |
| Final `dotnet build dotnet/MEVana.slnx` after both archives | Build succeeded, **0 warnings, 0 errors**. |
| Final `dotnet test dotnet/MEVana.slnx --verbosity quiet` after both archives | **Passed 194, Failed 0, Skipped 0, Total 194**. |
| Final `openspec validate --all --strict` after both archives | **5 passed, 0 failed**: catalog, request, result, ranking, outcome. |
| `git diff --check` and `git diff --cached --check` throughout logical commits | Exit 0, no output, including staged new implementation, fixtures and archive files. |
| `git fetch origin`; `git log --oneline main..origin/main`; `git log --oneline --left-right main...origin/main` | No remote-only commits. Remote tip was `e4ae5f488309ae579d2bddcfca1a8b65925c2fc4`. No merge was needed. |
| `git show --stat --oneline` for concurrent `53efc4f9`, `0faefd6c`, `4ca3729f` | Each touched only `docs/2026-10-28-0835-CDT-research-handoff-09-read-only-geyser-capture.md`; all preserved. |
| `git merge-base --is-ancestor origin/main main` before implementation push | Exit 0; fast-forward ancestry established. |
| `git diff --name-status 7ab2821d -- dotnet/src/MEVana.Research.Api dotnet/src/MEVana.Research.Persistence crates Dockerfile docker-compose.yml` | Exit 0, no output. |
| `openspec list --json` after archives | `changes: []`. |
| `git push origin main` for implementation and prior logical units | Exit 0; fast-forward `e4ae5f48..4ca3729f main -> main`, no force push. |
| Post-implementation `git rev-parse HEAD origin/main` and `git ls-remote origin refs/heads/main` | All equal `4ca3729fa50eb71837f8c4d82872f2a6fd365af8`. |
| `git status --short` after implementation publication | Clean, no output. |

Read-only inspection also used `rg`, `cat`, `sed`, existing tests/closeout, generated workflow instructions and Git history/diffs. File creation and archive checks used local Python and shell writes. There were no runtime provider or market-data calls; Git network access was solely for the requested repository synchronization.

## Design choices beyond the handoff

- New helpers reuse `CatalogRules.Id` and the existing zero-offset UTC rule without modifying prior contracts. Shared shape and catalog helpers keep both new contracts consistent. No existing enums or `ValidationIssue` were redefined.
- Ranking has a fixed WATCH kind, schemaVersion 1 input and canonicalizationVersion 1 output. Required numeric fields are nullable until validation. Training may exceed 24 hours if it fits coverage; only validity TTL is capped at 24 hours.
- Valid-from is inclusive; expiry is exclusive. A non-UTC `nowUtc` yields `invalid_utc`; `nowUtc == expiresAtUtc` yields `expired`. Time differences avoid adding durations to timestamps at numeric boundaries.
- Route equality is exact ordered token and pool sequence equality, with ordinal identifiers. Reversals/rotations are not normalized. Adjacent equal tokens cannot match a distinct-token catalog pool. Rankings sort entries only by rank; routes never sort.
- Outcome schemaVersion 1 and dataset/metadata/strategy provenance make catalog validation explicit. Detection must lie within dataset coverage, including boundaries; later decision/reconciliation can extend beyond coverage. As with results, historical validation does not depend on current `VerifiedAvailable`.
- Outcome decision time is required. Equal event timestamps are valid. Landed/Failed/Dropped are exactly the submitted states; Expired denotes non-submitted expiry, while a lost/unconfirmed submission uses Dropped. Non-submitted states forbid submission time, signatures and costs. Landed slot is forbidden outside Landed/Failed. Optional reason codes on other states remain permitted if valid.
- Costs use nullable `ulong`, including exact values above long.MaxValue. Profit quantities remain independent nullable `long`, including negative failed-live fee losses. No cost summation or profit fallback/derivation occurs.
- Ranking links are paired ID/rank declarations, not lookups; no artifact/result store exists in this slice. Unranked outcomes are accepted for every policy. Source/experiment IDs are checked syntactically only.
- Idempotency comparison is a pure exact canonical-content check under ordinal keys; it performs no reservation, storage or durable exactly-once processing. It rejects invalid inputs before comparing.
- Mandatory OpenSpec artifacts use the schema-required locations; the dated general plan is in `docs/2026-10-28-0810-CDT-research-stage0-handoff-08-implementation-plan.md`. The handoff explicitly authorized the complete proposal/apply/archive sequence and both CLI archive commands. All generated main specs were dated after verifying the merge.

## Existing behavior and proposed follow-ups

All pre-existing contract source files are unchanged. In particular:

1. The **existing request bound uses the second token's maximum**. Propose a separate OpenSpec change to align it with the first-token quote convention, with explicit compatibility and canonical-request implications.
2. Existing request properties ending in **`Utc` accept nonzero offsets**. Propose a separate OpenSpec change deciding whether to enforce zero-offset UTC at syntax validation and documenting any effect on previously accepted requests.

Neither follow-up was implemented here. All new ranking/outcome timestamps require zero-offset UTC, and both use first-token quote bounds as required.

## Commits and publication

Handoff 08 logical units:

- `bcc45a86` — standing OpenSpec context, separately validated and committed.
- `1e1d80b1` — dated local plan and ranking proposal/design/spec/tasks.
- `c336355a` — ranking implementation, tests, fixture and archive.
- `010b2bb8` — outcome proposal/design/spec/tasks.
- `88ee7da4` — outcome implementation, tests, two fixtures and archive.

During execution, three other local documentation commits appeared: `53efc4f9`, `0faefd6c` and `4ca3729f`. They were inspected, preserved and included in the fast-forward publication; this task did not rewrite them or execute handoff 09. No remote-only commits appeared, so there was no reconciliation merge.

The implementation and original handoff are published and verified synchronized at `4ca3729fa50eb71837f8c4d82872f2a6fd365af8`. This closeout is a final separate documentation commit. The final publication gate fetches again, inspects any remote-only history, pushes without force, then checks `HEAD`, `origin/main` and GitHub main for equality and a clean working tree. Its post-commit result is reported in the task's final response.

## Scope preserved

No changes were made to Rust crates/trading logic, Docker/deployment configuration, MEVana-UI, API host or persistence. No API keys, passwords, environment files or wallet material were deleted, rotated or rewritten, and no credentials were committed. No trading process or live-data/provider operation was started. Trading remains unactivated by this work. API/UI/Geyser implementation and all later slices remain outside handoff 08.
