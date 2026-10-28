# 2026-10-27 23:22 CDT — Research Stage 0 handoff 07 closeout

## Outcome

OpenSpec 1.15.0 is initialized for Codex and Claude. The dataset catalog and experiment request are baselined without changing their existing code. The experiment result contract is specified, implemented in Core, tested and archived into `openspec/specs/research-experiment-result/spec.md`.

The solution passes **79 tests, 0 failures, 0 skipped**, versus the verified entry baseline of **40**: **39 additional passing tests**. The build has **0 warnings and 0 errors**. All three main specs pass strict validation.

The fixture `dotnet/tests/MEVana.Research.Tests/Fixtures/2026-10-27-synthetic-experiment-result-v1.json` is **Synthetic contract test data**, independently authored using Python standard-library JSON and SHA-256 and compared byte-for-byte with C# canonical output. It is not market evidence and establishes no strategy performance, replay correctness or live profitability.

## Material commands and exact results

All commands ran from the MEVana repository root; `openspec` below is `~/.local/bin/openspec`.

| Command | Result |
| --- | --- |
| `git status --short` at entry | Exit 0, no output: clean tree. |
| `git log -3 --oneline` at entry | `175aa1ea` handoff, `c2ca1145` comparison, `a3f604ce` Rust stabilization. |
| `openspec init --tools codex,claude --no-animation` | Exit 0; structure created; Codex and Claude setup complete; schema `spec-driven`. |
| `git ls-remote origin refs/heads/main` at entry | Exit 0; `22161d01998bb1339e09396ab4d21f6be5004e76`. |
| `git fetch origin main` and `git log --oneline --left-right main...origin/main` | Exit 0; local-only `175aa1ea`, remote-only `22161d01`. |
| `git show --stat origin/main` and `git diff --name-only c2ca1145..origin/main` | Remote adds only `docs/2026-10-17-mevana-arbitrage-research-intelligence-service.md` (432 lines). |
| `git merge --no-edit origin/main` after scaffold commit | Exit 0; clean `ort` merge `8f76b23a`, preserving both histories. |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet` before implementation | Exit 0; Passed 40, Failed 0, Skipped 0, Total 40. |
| `openspec context --json` and `openspec list --json` | Exit 0; resolved root `/Users/michaelinman/Repos/MEVana`; initially no active changes. |
| `openspec new change add-research-experiment-result` | Exit 0; created with `spec-driven` schema. |
| First `openspec validate --specs --strict` | Exit 1; 1 passed, 1 failed: request syntax requirement exceeded 500 characters. Split the documentation requirement without changing behavior. |
| Repeated `openspec validate --specs --strict` | Exit 0; 2 passed, 0 failed. |
| `openspec list --specs` and `openspec show research-experiment-request --type spec --json --no-scenarios` | Exit 0; catalog has 3 requirements, request has 5. Full baseline files were also read. |
| `openspec instructions proposal/specs/design/tasks --change add-research-experiment-result --json` (each artifact separately) | Exit 0; used returned templates/dependencies and repo-local paths. |
| `openspec status --change add-research-experiment-result --json` during authoring | Exit 0; all four planning artifacts became `done`. |
| `openspec validate --all --strict` after proposal | Exit 0; 3 passed, 0 failed: 1 change and 2 baseline specs. |
| `openspec instructions apply --change add-research-experiment-result --json` | Exit 0; tracked progress advanced from 0/2 to 1/2 to 2/2; final state `all_done`. |
| First `dotnet test dotnet/MEVana.slnx --verbosity quiet --filter FullyQualifiedName~ExperimentResultContractTests` | Exit 1; Passed 38, Failed 1, Total 39. xUnit could not convert an InlineData Int32 to nullable Int64. |
| `dotnet test dotnet/MEVana.slnx --no-restore --filter FullyQualifiedName~RequiredCountValidation --logger 'console;verbosity=normal'` | Exit 1; Passed 1, Failed 1, Total 2; confirmed test invocation type mismatch. Corrected literal `-1` to `-1L`. |
| Repeated focused result test command | Exit 0; Passed 39, Failed 0, Skipped 0, Total 39. |
| `dotnet build dotnet/MEVana.slnx` before archive | Exit 0; build succeeded; 0 warnings, 0 errors. |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet` before archive | Exit 0; Passed 79, Failed 0, Skipped 0, Total 79. |
| `openspec validate --all --strict` before archive | Exit 0; 3 passed, 0 failed. |
| `git diff --check` before archive | Exit 0, no output. |
| `openspec instructions archive --change add-research-experiment-result --json` | Exit 0; root resolved, no additional guidance. |
| `openspec list --json` before archive | Exit 0; completedTasks 2, totalTasks 2, status complete. |
| `openspec archive add-research-experiment-result --yes` | Exit 0; task status Complete; created result main spec; +8 requirements, 0 modified/removed/renamed; archived to `openspec/changes/archive/2026-10-27-add-research-experiment-result/`. `--yes` supplies the archive already authorized by the handoff. |
| Python archive-content assertions | Exit 0; title correct; purpose and all eight requirement blocks exactly match delta; no delta-operation headings remain. Added the main spec's date after CLI generation. |
| `dotnet build dotnet/MEVana.slnx` after archive | Exit 0; build succeeded; 0 warnings, 0 errors. |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet` after archive | Exit 0; Passed 79, Failed 0, Skipped 0, Total 79. |
| `openspec validate --all --strict` after archive | Exit 0; 3 passed, 0 failed: catalog, request and result main specs. |
| `git diff --check` and `git diff --cached --check` | Exit 0, no output, including staged implementation and archived artifacts. |
| Targeted `git diff --name-only c2ca1145 --` for the existing catalog and four request contract files | Exit 0, no output: baseline code unchanged. |
| GitHub MCP `github_search_branches` for `mike-inman/MEVana`, query `main` | Successful; returned main and integration branch names. Git transport supplied authoritative commit IDs. |
| `git merge-base --is-ancestor origin/main main` | Exit 0: the observed remote tip is an ancestor of the implementation. |
| `git ls-remote origin refs/heads/main` immediately before implementation push | Still `22161d01998bb1339e09396ab4d21f6be5004e76`. |
| `git push origin main` for implementation and prior logical units | Exit 0; fast-forward `22161d01..e025c988 main -> main`. No force push. |
| `git rev-parse HEAD` and `git ls-remote origin refs/heads/main` after implementation push | Both `e025c988329866a8b7721e7fe5049526dab19f6c`. |
| `git status --short` after implementation push | Exit 0, no output. |
| `openspec list --json` after archive/publication | Exit 0; `changes: []`. |

Read-only inspection also used `rg`, `cat`, `sed`, Git history/diff/status, and the generated workflow skills. Artifact creation used local file writes. No runtime provider or market-data calls were made.

## Baseline spec versus code findings

There were no pre-existing OpenSpec specs to reconcile. The baseline specs record current constructors, validators, canonicalization and tests; no baseline contract behavior was repaired.

- Request timestamp property names end in `Utc`, but request syntax and catalog validators accept nonzero offsets, comparing instants. Canonicalization converts those instants to UTC. Dataset coverage constructors instead require zero offsets.
- A missing token in request catalog validation short-circuits pool and trade-bound checks after independent availability, strategy and window checks. The baseline preserves this ordering.
- Request quote bounds use the second token's maximum and the dataset maximum; reversing a token pair can change admissibility. The first token's maximum does not govern quote bounds.
- Request canonicalization checks syntax only, so a well-formed request can canonicalize without a known or available dataset. Pair order is significant, pool order is not.
- Catalog provenance/suitability are local descriptive text, not verified provenance hashes or availability checks. `VerifiedAvailable` is declared metadata. These limitations remain visible and unchanged.

## Design choices beyond the minimum handoff

- Result windows, as well as creation time, require zero-offset UTC and are limited to 24 hours. Result/metadata/engine/cost/error/reason identifiers use catalog-style 1–64 ASCII identifiers. Request fingerprint version must be 1, with lowercase hexadecimal SHA-256.
- Policies hold no separate dataset/window/cost-model fields, preventing scope divergence structurally.
- Raw required counts are nullable to distinguish omission from zero. Added `SelectedOrExecuted` for the accounting inequality. Rejection reasons are unique by ordinal comparison; duplicates are rejected. Accounting subtracts from the evaluated budget to avoid long overflow. No additional detected/missed-count relationship is assumed beyond nonnegativity and the required rejected-plus-selected inequality.
- Failed/unavailable results accept null or empty policy collections and require an error code. Completed results reject any error code and require exactly all three policies; individual permitted profit fields may remain unknown.
- Result catalog validation intentionally accepts historical results even if current `VerifiedAvailable` is false. It still requires dataset identity, exact metadata version, supported strategy and coverage bounds.
- `RequestFingerprint.FromRequest` hashes the actual UTF-8 canonical request content. The three-argument result validator also checks that hash and dataset/strategy/window binding against the original request. The two-argument catalog validator checks declared hash shape and catalog compatibility only; neither proves truth of evidence labels or measurements.
- Canonical policies sort by declaration order, reasons by ordinal code. All nullable fields are explicit; integers retain full long precision. Failed null policies and empty policies remain distinct JSON values.
- The OpenSpec design and tasks use the schema-mandated locations. Generated workflow Markdown and all authored documents carry dates. The generated proposal skill normally separates planning and apply turns; this handoff explicitly authorized the full propose/implement/archive sequence, which was followed. CLI archive performed its own synchronous spec merge, followed by explicit content verification.

## Requirement coverage

| Result requirement | Test coverage in `ExperimentResultContractTests` |
| --- | --- |
| Raw immutable submissions | `RawNullInvalidDuplicateAndOffsetInputsAreRetained` |
| Identity and provenance | `IdentityValidation`, `IdentifierBoundaries`, `HashValidation`, `RequestVersionValidation`, `UtcAndWindowBoundaries`, `FingerprintHashesActualCanonicalUtf8AndVerifiesScope` |
| Evidence and terminal status | `ValidSyntheticResultAndHistoricalAvailability`, `NullAndUndefinedEnums`, `NonCompletedHasErrorAndNoMetrics`, `CompletedHasEveryPolicyExactlyOnceAndNoError` |
| Independent signed profit quantities | `ProfitsStayIndependentIncludingNegativeAndLongBoundaries`, `RealizedRequiresLive`, `SyntheticForbidsSimulatedEvenZero` |
| Complete common-scope comparison | `CompletedHasEveryPolicyExactlyOnceAndNoError`, `UtcAndWindowBoundaries`, `CanonicalOrderingCultureExactBytesAndSharedScope` |
| Candidate accounting | `RequiredCountValidation`, `OptionalMissedAndZeroCounts`, `RejectionReasonsAndMissingLists`, `AccountingIsOverflowSafe` |
| Catalog semantics | `SyntaxPrecedesLookupAndIssuesAreImmutableDeterministic`, `CatalogIdentityMetadataStrategyAndCoverage`, `ValidSyntheticResultAndHistoricalAvailability` |
| Version one canonical results | `CanonicalOrderingCultureExactBytesAndSharedScope`, `InvalidResultsCannotCanonicalize`, `GoldenFixtureIsExplicitlySyntheticContractData`, profit/status tests |

## Commits and publication

Logical units committed before this closeout:

- `fcdfc2bc` — initialize OpenSpec scaffolding.
- `8f76b23a` — preserve the unexpected remote documentation commit with a local merge.
- `58ee2263` — baseline catalog/request specs.
- `48687035` — proposal, design, spec delta and tasks.
- `e025c988` — implementation, tests, synthetic fixture and archive.

All of those commits, including the original handoff commit, were fast-forward pushed and verified synchronized at `e025c988329866a8b7721e7fe5049526dab19f6c` before authoring this closeout. The closeout is published in a final separate documentation commit. Final publication checks require the observed remote tip to remain the expected implementation commit before that push, then require `HEAD`, `origin/main` and GitHub `main` to agree and the working tree to be clean. The final task response reports that post-commit verification.

## Scope preserved

No Rust crates, trading logic, Docker/deployment configuration, separate MEVana-UI checkout, API host or persistence implementation changed. No API keys, passwords, environment files or wallet material were deleted, rotated or rewritten. No credentials were added. No trading or live-data process was launched; trading remains untouched and unactivated by this work. Handoffs 08–10 remain outside this slice.
