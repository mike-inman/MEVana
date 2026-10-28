# 2026-10-27 23:09 CDT — Handoff 07: OpenSpec setup and experiment result contract

## Purpose

Begin the Stage 0 MVP defined in `docs/2026-10-06-2328-CT-MEVANA-Azure-AI-Rearchitecture-Assessment.md` (sections 7.2, 9 and 13). Stage 0 fixes versioned contracts before any market data or model exists, so that a later answer to "does route ranking beat simple baselines?" cannot be distorted by blurred profit fields, missing provenance or omitted rejected candidates.

This handoff covers one slice: set up OpenSpec, record the two existing contracts as baseline specs, and add the **experiment result** contract through an OpenSpec change. Do not implement the other slices listed under "Later slices".

## Entry state

- MEVana `main` is at `c2ca1145` plus the commit that adds this handoff. That commit may not yet be on GitHub; push it together with your work in step 6.
- The C# solution is `dotnet/MEVana.slnx` (net10.0, xUnit). The last recorded run on 2026-10-16 was `dotnet test dotnet/MEVana.slnx --verbosity quiet`, which passed 40 tests.
- The existing contracts live in `dotnet/src/MEVana.Research.Core/`:
  - `DatasetCatalog.cs` contains `TokenMetadata`, `PoolMetadata`, `DatasetMetadata` and `ResearchCatalog`. It validates in its constructors through `CatalogRules`, using identifiers of up to 64 characters drawn from `[A-Za-z0-9_-]`, UTC-only coverage times and positive atom bounds.
  - In `ExperimentRequest.cs`, the raw DTO deliberately keeps invalid, null and duplicate values as they were submitted. Three classes act on it:
    - `ExperimentRequestSyntaxValidator` returns `ValidationIssue(Field, Code)` records.
    - `ExperimentRequestValidator` validates a request against the catalog.
    - `ExperimentRequestCanonicalizer` writes version-1 canonical JSON with sorted pool IDs and ISO-8601 UTC times.
  - The discovery contracts are `DiscoveryCandidate.cs` and `DiscoveryEvidence.cs`.
- `dotnet/src/MEVana.Research.Api/Program.cs` is an empty host, and `MEVana.Research.Persistence` has no source files. Do not build out either one in this slice.
- OpenSpec CLI 1.15.0 is installed at `~/.local/bin/openspec`. No `openspec/` directory exists yet.
- The separate React UI (`/Users/michaelinman/Repos/MEVana-UI`, Next.js 16 / React 19) is clean and synced with GitHub at `2b66455`. Do not modify it in this slice.

## Work

1. **Initialize OpenSpec.** In the MEVana repository root, run `openspec init --tools codex,claude --no-animation`. Read the generated instruction files and follow the workflow they describe for this version; their conventions take precedence over any assumption in this handoff. Commit the generated scaffolding on its own.
2. **Write baseline specs for the existing contracts, without changing code.** Create capability specs named `research-dataset-catalog` and `research-experiment-request`. Derive every requirement and WHEN/THEN scenario from the current code and its tests in `dotnet/tests/MEVana.Research.Tests/`. Record what the code actually does today, even where that behavior looks questionable, and note questionable behavior in the closeout instead of changing it. Run `openspec validate --specs --strict`.
3. **Propose the experiment result contract** as the OpenSpec change `add-research-experiment-result`, containing `proposal.md`, `tasks.md` and a spec delta for a new capability `research-experiment-result`. The contract must include the requirements below. Field names are suggestions; the rules are mandatory.
   - **Identity and provenance:** the result must carry:
     - a result ID;
     - a SHA-256 hash of the `ExperimentRequestCanonicalizer` content, along with its canonicalization version;
     - the dataset ID and dataset `MetadataVersion`;
     - the strategy version, the engine version and the cost-model version;
     - creation time in UTC only.
   - **Evidence kind:** an explicit enum with the values `Synthetic`, `Simulated` and `Live`, plus a status with the values `Completed`, `Failed` and `Unavailable`. A failed or unavailable result must carry an error code and no metric values.
   - **Separate profit fields:** each policy result carries three separate nullable signed quantities, all measured in quote atoms (`long`) and allowed to be negative: estimated, simulated and realized net profit.
     - Realized profit must be null unless the evidence kind is `Live`.
     - Simulated profit must be null when the evidence kind is `Synthetic`.
     - No constructor or mapper may copy one of these fields into another.
   - **Policy comparison:** a completed result contains one entry for each policy kind: `FixedUniverse`, `SimpleRules` and `ModelRanking`.
     - Each policy kind appears once, and the result is rejected if a kind is missing.
     - All policies share the result's single dataset, window and cost-model version.
     - Each entry reports:
       - the number of candidates evaluated;
       - the number of opportunities detected;
       - rejected candidates counted by reason code;
       - missed candidates, when known.
       Counts are non-negative, and rejected plus executed-or-selected never exceeds evaluated.
   - **Canonical form:** a version-1 canonical JSON for the result, deterministic and following the request canonicalizer's style. Two results that differ only in input ordering must produce identical bytes.
   - **Validation style:** follow the existing request pattern. That means a raw DTO that keeps the input exactly as submitted, a syntax validator that returns `ValidationIssue` codes, and a semantic validator checked against the `ResearchCatalog` (unknown dataset, metadata version mismatch, strategy version not supported by the dataset).
4. **Implement the change** in `MEVana.Research.Core`, adding tests in `MEVana.Research.Tests` that cover every requirement and scenario. Add one golden fixture of a version-1 canonical result under `dotnet/tests/MEVana.Research.Tests/Fixtures/`.
   - The fixture's evidence kind must be `Synthetic`.
   - Its identifiers must say so (for example, a dataset ID starting with `synthetic-`).
   - It must never be described anywhere as market evidence.
5. **Validate:**
   - `dotnet build dotnet/MEVana.slnx`
   - `dotnet test dotnet/MEVana.slnx --verbosity quiet`; report the exact pass count and compare it with the baseline of 40
   - `openspec validate --all --strict`
   - `git diff --check`

   When all of these pass, archive the change with `openspec archive add-research-experiment-result` so that the delta merges into `openspec/specs/`, then validate again.
6. **Write the closeout and push.** Write a dated closeout named `docs/2026-10-XX-HHMM-CDT-research-stage0-handoff-07-closeout.md`. It must record:
   - the commands run, with their exact results;
   - any spec-versus-code discrepancies found in step 2;
   - any design choices made beyond this handoff.

   Commit in logical units: scaffolding, baseline specs, the change proposal, then the implementation and archive. Before pushing, confirm that GitHub `main` still points at the expected commit, then fast-forward push. Never force push.

## Constraints

- Make no changes to Rust crates, trading logic, Docker or deployment configuration, or MEVana-UI.
- Do not delete, rotate or rewrite any API keys, local passwords, `.env` files or wallet material. Commit no credentials.
- Keep trading disabled. Nothing in this slice reads live market data or calls external services.
- Make sure no work is lost: every change must be committed and pushed before the closeout is considered done.
- Date every document you create.

## Later slices (not in scope)

- **Handoff 08:** ranking artifact and execution outcome contracts, using the same OpenSpec flow.
- **Handoff 09:** a minimal read-only Research API in `MEVana.Research.Api`. It would serve the catalog, experiment requests and results from an in-memory store seeded with the synthetic fixtures.
- **Handoff 10:** a research page in MEVana-UI (React/Next.js). It would list datasets and experiments, and display a result comparing `ModelRanking` against the `FixedUniverse` and `SimpleRules` baselines, with estimated, simulated and realized profit in separate columns and the evidence kind shown prominently. It would also add a `research` service endpoint to `src/lib/config.ts`.

## Exit criteria

- OpenSpec is initialized.
- The baseline specs match the existing contract behavior.
- `research-experiment-result` is specified, implemented, tested, archived into `openspec/specs/`, and passes strict validation.
- The .NET tests pass, with a reported count above 40.
- The closeout is committed, and local and GitHub `main` are synchronized.

A passing build alone must not be described as passing tests, and synthetic fixtures must not be described as market evidence.
