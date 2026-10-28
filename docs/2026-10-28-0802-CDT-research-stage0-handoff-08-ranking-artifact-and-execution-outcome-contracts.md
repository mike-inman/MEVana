# 2026-10-28 08:02 CDT — Handoff 08: Ranking artifact and execution outcome contracts

## Purpose

Complete the Stage 0 contract set from `docs/2026-10-06-2328-CT-MEVANA-Azure-AI-Rearchitecture-Assessment.md` (section 9, Stage 0). Handoff 07 delivered the dataset catalog, experiment request and experiment result contracts. This slice adds the two remaining contracts:

1. **Ranking artifact:** the advisory output of a policy, telling the Rust engine which routes deserve attention. It merges the 10-06 "ranking-artifact" with the WATCH message from section 8 of `docs/2026-10-17-mevana-arbitrage-research-intelligence-service.md` into **one** contract.
2. **Execution outcome:** what actually happened to a candidate, whether it was rejected, shadowed, paper-traded or submitted live. This contract fixes finding F3 of the 10-06 assessment, which found that the hotpath records intent and estimated profit as though they were outcomes.

These two contracts determine whether a later comparison between the ranking model and the simple baselines can be trusted. Prefer strictness wherever the rules below leave a choice open.

## Entry state

- MEVana `main` is at `7ab2821d` plus the commit that adds this handoff. That commit may not yet be on GitHub; push it together with your work.
- OpenSpec 1.15.0 is initialized and has no active changes. The main specs are `research-dataset-catalog`, `research-experiment-request` and `research-experiment-result`, and all three pass `openspec validate --all --strict`.
- `dotnet test dotnet/MEVana.slnx --verbosity quiet` passes 79 tests, the baseline for this slice.
- Reuse the existing types and patterns in `dotnet/src/MEVana.Research.Core/`:
  - `EvidenceKind` (`Synthetic`, `Simulated`, `Live`) and `PolicyKind` (`FixedUniverse`, `SimpleRules`, `ModelRanking`);
  - `ValidationIssue`;
  - the identifier and UTC rules in `CatalogRules`;
  - the split into a raw DTO that keeps input exactly as submitted, a syntax validator, a semantic validator checked against the catalog, and a version-1 canonicalizer;
  - the golden fixture approach in `dotnet/tests/MEVana.Research.Tests/Fixtures/`.

  Do not redefine any of these. Follow `docs/2026-10-27-2322-CDT-research-stage0-handoff-07-closeout.md` for conventions such as nullable required counts and overflow-safe accounting.

## Scope decisions already made

- **Solana DEX routes only.** The 10-17 document's CEX, cross-chain, oracle and funding strategies are out of scope, so neither contract gets fields for them.
- **Integers only.** Amounts are signed or unsigned `long` quote atoms. Probabilities and confidences are integer basis points from 0 to 10000. No floating point appears in either contract.
- **WATCH only.** Do not define TRIGGER messages; they belong to the latency-sensitive path and a later phase.
- **Advisory only.** Neither contract may contain any of the following: signing material, wallet identifiers, serialized transactions, instructions, or any field that could authorize a trade. Spec this as a requirement and test it.

## Shared route shape

Both contracts use one route type, defined once in Core and specified in the first change below.

- A route is an ordered list of token IDs together with an ordered list of pool IDs.
- The route is a **closed cycle**: the first token equals the last.
- It contains between 3 and 5 tokens, which means 2 to 4 hops.
- The number of pools equals the number of tokens minus one, and pool IDs within a route are distinct.
- Each pool `i` must contain tokens `i` and `i+1`. Check this against the catalog; the syntax validator checks shape only.
- Direction is implied by the order of the tokens.

## Work

1. **Add project context to `openspec/config.yaml`.** Record the standing constraints above, so future OpenSpec artifacts inherit them:
   - Solana DEX scope;
   - integer amounts and basis points;
   - UTC zero-offset times;
   - synthetic fixtures must be labeled `Synthetic` and never presented as market evidence;
   - no trading authority in research contracts;
   - dated documents.

   Validate, then commit this on its own.
2. **Create and implement the OpenSpec change `add-research-ranking-artifact`** (new capability `research-ranking-artifact`). Its requirements:
   - **Identity and provenance:**
     - artifact ID and schema/canonicalization version 1;
     - producing `PolicyKind` and `EvidenceKind`;
     - strategy version and feature-set version;
     - model version, required when the policy kind is `ModelRanking` and null otherwise;
     - the training or source dataset ID with its exact `MetadataVersion`;
     - the training window (UTC, increasing);
     - an optional source experiment result ID.
   - **Point-in-time validity:**
     - an `asOfUtc` time, and an `observedSlot` greater than or equal to 0;
     - `validFromUtc` no earlier than `asOfUtc`;
     - `expiresAtUtc` after `validFromUtc`, at most 24 hours later.
     - The training window must end no later than `asOfUtc`, which prevents the model from training on future data.
     - Provide a validator overload that takes a `nowUtc` and returns `expired` or `not_yet_valid`.
   - **Entries:** between 1 and 1000. Each entry has:
     - a rank, with ranks contiguous from 1 and unique;
     - a route;
     - a size region of positive quote-atom bounds, with minimum no greater than maximum;
     - a monitoring priority from 0 to 100;
     - a research confidence in basis points;
     - an optional calibrated success probability in basis points, kept as a **separate** field from research confidence;
     - a sample count greater than or equal to 0;
     - an optional estimated net value in signed quote atoms.

     A ranking artifact never contains simulated or realized profit. Duplicate routes (identical token and pool sequences) are rejected.
   - **Catalog semantics:** apply the dataset, metadata version and strategy checks from the result contract. In addition, every route token and pool must exist in the dataset, the pools must match their adjacent tokens, and size bounds must not exceed the dataset's maximum or the quote token's maximum. The quote token is the route's first token. Unlike the existing request contract, do not use the second token's maximum.
   - **Version-1 canonical form:** entries sorted by rank, deterministic bytes, plus a dated synthetic golden fixture whose IDs start with `synthetic-`.
3. **Create and implement the OpenSpec change `add-research-execution-outcome`** (new capability `research-execution-outcome`). Its requirements:
   - **Identity:** outcome ID and an idempotency key (identifier rules). Two outcomes with the same idempotency key and different canonical bytes must be detectable; provide a helper and test it.
   - **Linkage:** a policy kind, plus an optional ranking artifact ID and entry rank, which must be either both present or both absent. Outcomes for candidates **outside** any ranking (exploratory or baseline coverage) must be representable, because dropping them would bias the comparison. There is also an optional experiment result ID.
   - **Mode and evidence:**
     - `ExecutionMode` is one of `Shadow` (never submitted), `Paper` or `LiveSubmission`.
     - `EvidenceKind.Live` is allowed only when the mode is `LiveSubmission`, and `LiveSubmission` requires `Live` evidence.
     - `Synthetic` evidence is allowed only with `Shadow` or `Paper`.
   - **Candidate facts:** the route, the input size in quote atoms (positive), `observedSlot`, and `detectedAtUtc`.
   - **Terminal state:** one of `Rejected`, `NotSubmitted`, `Landed`, `Failed`, `Dropped` or `Expired`.
     - `Rejected`, `Failed`, `Dropped` and `Expired` require a reason code.
     - `Landed`, `Failed` and `Dropped` are submitted states. They require `submittedAtUtc` and are allowed only in `Paper` or `LiveSubmission` mode.
     - `Landed` and `Failed` also require a landed slot greater than or equal to `observedSlot`.
     - In `LiveSubmission` mode, submitted states also require a transaction signature: a base58 string of 64 to 88 characters, syntax-checked only.
   - **Monotonic timing:** detection, decision, submission and reconciliation times, where present, must not decrease. Every time is zero-offset UTC.
   - **Separate profits** (signed nullable quote atoms):
     - Estimated profit is the value at decision time.
     - Simulated profit is forbidden for `Synthetic` evidence.
     - Realized profit is allowed only for `LiveSubmission` outcomes in state `Landed` or `Failed`, and requires a `reconciledAtUtc`. A failed live trade may realize a negative amount, for fees.
     - Explicit cost fields cover the network fee, the priority fee and the tip. They are unsigned and nullable, and must be null when not submitted.
     - **Never** derive realized profit from estimated or simulated values. Spec this and test it.
   - **Catalog semantics** apply as in the ranking artifact.
   - **Version-1 canonical form** and a synthetic golden fixture, covering at least one `Rejected` outcome and one `Landed` outcome in `Paper` mode.
4. **For each change:** write `proposal.md`, `design.md`, `tasks.md` and the spec delta, then validate strictly. Implement the change, writing tests for every requirement and scenario, and archive it with `openspec archive <change> --yes`. Then confirm that the merged main spec matches the delta. Do the ranking artifact change first, because the outcome contract reuses its route type.
5. **Validate:**
   - `dotnet build dotnet/MEVana.slnx`, which must show 0 warnings;
   - `dotnet test dotnet/MEVana.slnx --verbosity quiet`; report the exact count against the baseline of 79;
   - `openspec validate --all --strict`, with 5 specs passing;
   - `git diff --check`.

   Confirm that the five existing contract source files are unchanged since `7ab2821d`.
6. **Write the closeout and push.** Write `docs/2026-10-XX-HHMM-CDT-research-stage0-handoff-08-closeout.md`. Include:
   - the commands run and their exact results;
   - the design choices made beyond this handoff;
   - a short "Stage 0 complete" summary listing all five contracts and their specs.

   Commit in logical units. Before pushing, run `git fetch` and inspect any commits that are only on the remote. If any exist, merge them without discarding anything, as in handoff 07, and record that in the closeout. Push as a fast-forward, never force. Finish with `HEAD`, `origin/main` and GitHub `main` identical and the working tree clean.

## Constraints

- Make no changes to Rust crates, trading logic, Docker or deployment configuration, MEVana-UI, the API host or persistence.
- Do not delete, rotate or rewrite any API keys, local passwords, `.env` files or wallet material. Commit no credentials.
- Keep trading disabled. Make no live-data or external-service calls.
- Do not change existing contract behavior, including the two oddities recorded in handoff 07: the request bound uses the second token's maximum, and the `...Utc` request fields accept non-UTC offsets. List both in the closeout as proposed follow-up changes.
- Date every document. Make sure no work is lost.

## Later slices (not in scope)

- **Handoff 09:** a minimal read-only Research API in `MEVana.Research.Api`. It would serve the catalog, requests, results, ranking artifacts and outcomes from an in-memory store seeded with the synthetic fixtures. It would label the evidence kind on every response and include contract tests.
- **Handoff 10:** a research page in MEVana-UI (React/Next.js). It would show the comparison of `ModelRanking` against `FixedUniverse` and `SimpleRules`, with estimated, simulated and realized profit in separate columns, rejected and missed counts, and a prominent evidence-kind badge. It would also add a `research` endpoint to `src/lib/config.ts`.
- **Follow-up:** an OpenSpec change correcting the two request-contract oddities.

## Exit criteria

- `research-ranking-artifact` and `research-execution-outcome` are specified, implemented, tested, archived and strictly valid.
- All five Stage 0 specs pass `openspec validate --all --strict`.
- The .NET tests pass with a reported count above 79, and the build shows 0 warnings.
- Existing contract code is unchanged.
- The closeout is committed, and local and GitHub `main` are synchronized.

Synthetic fixtures must not be described as market evidence, and passing contract tests must not be described as evidence that ranking works.
