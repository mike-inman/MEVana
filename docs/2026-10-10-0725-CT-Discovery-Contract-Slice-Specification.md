# Solana DEX candidate and evidence contracts

Date: 2026-10-10 07:25 CT. Purpose: complete D1 context for local discovery-contract development after separate scope approval. Derived from the retained 2026-10-09-2138-CT specification draft; replaces no existing request profile. Solana was explicitly selected by the user. This document is the complete specification for this slice and is host-hash-pinned.

Scope: .NET 10 dependency-free MEVana.Research.Core and assigned tests only. No network/provider calls, quotes/cost calculations, matching, ranking, API/UI, database, dataset downloads, Rust, deployment, wallet signing, or trading. Preserve credentials and unrelated work. Synthetic test fixtures never prove live data, DEX support, executable routes, or profits. Existing exchange names do not imply validated adapters.

Own exactly DiscoveryEvidence.cs, DiscoveryCandidate.cs, DiscoveryContractTests.cs and MEVana.Research.Tests.csproj under the existing Core/test directories. Preserve existing tests and references when setting up the test project; no solution or Core project edits. Read all assigned inputs in completed responses before writing. Writes must fit source ceilings without dropping behavior or tests.

Namespace MEVana.Research.Core. Six public sealed immutable classes with ordinary public constructors, parameters in the exact following order, and corresponding PascalCase get-only properties of matching types:

```csharp
DiscoveryAsset(string? chainId, string? mintAddress)
DiscoveryVenue(string? dexId, string? protocolVariant, string? poolAddress)
DiscoveryObservation(string? sourceId, string? captureId,
    DateTimeOffset? observedAtUtc, ulong? slot)
DiscoveryQuote(DiscoveryAsset? inputAsset, DiscoveryAsset? outputAsset,
    ulong? inputAtoms, ulong? outputAtoms, DiscoveryVenue? venue,
    DiscoveryObservation? observation, bool? swapFeesIncluded)
DiscoveryCost(string? kind, DiscoveryAsset? asset, ulong? amountAtoms,
    DiscoveryObservation? observation)
DiscoveryCandidate(string? candidateId, DiscoveryQuote? firstLeg,
    DiscoveryQuote? secondLeg, IReadOnlyList<DiscoveryCost?>? additionalCosts,
    IReadOnlyList<string?>? evidenceIssues)
```

First five classes belong in DiscoveryEvidence.cs; DiscoveryCandidate belongs in DiscoveryCandidate.cs. No public setters, init setters, mutable fields or collection backing storage. Classes are evidence carriers, not validity certificates. Preserve all null, blank, malformed, unexpected-chain, differently cased and inconsistent values unchanged. Do not reject, trim, case-fold, normalize offsets, fabricate evidence, clamp amounts, or decide eligibility. UInt64 nullable amounts are exact asset atoms, including zero and UInt64.MaxValue; no floats or currency conversions. Negative external amounts are a future parser concern and cannot be represented by these typed fields.

Asset identity records chain/mint, never substitutes a symbol. Venue records DEX/protocol variant/pool. Retain quote direction, amounts, evidence and nullable fee-inclusion flag; unknown stays unknown. Retain original observation offset and absent slot. A slot/time does not prove simultaneous executability. Costs are separately itemized with asset/source, never aggregated, converted or filled with zero. Preserve first/second leg references and order even for inconsistent legs: subsequent evaluators check output/input asset and amount compatibility. No profit, rank, eligibility or executable flag belongs here.

Null lists stay null. Non-null lists must be defensively copied and exposed as truly read-only snapshots; caller array/list mutations and casts must not mutate them. Preserve order and duplicate/null elements. Cost instances are themselves immutable, so reference copying suffices. An empty issue list does not certify validity.

Test project: net10.0, Nullable enable, ImplicitUsings enable, IsTestProject true, IsPackable false; Microsoft.NET.Test.Sdk 17.14.1, xunit 2.9.3, xunit.runner.visualstudio 3.1.4 with PrivateAssets all; Using Xunit; relative ../../src/MEVana.Research.Core/MEVana.Research.Core.csproj only. No other dependencies. Preserve verified predecessor tests.

Meaningful tests must cover null/raw/malformed and unexpected-chain retention; distinct mint identities; all get-only surfaces; exact zero, UInt64.MaxValue and integers beyond double precision; missing amounts/fee flags/slots; non-UTC offsets; directed/ordered deliberately inconsistent legs retained; separate costs retaining units/source; null lists; ordered duplicate/null elements; caller mutations; and inability to mutate exposed snapshots. Tests must not demand validation, quoting, ranking, or fabricated profitability. No file-existence-only tests or source-string assertions as acceptance.

Require the five fixed host checks: SDK, solution listing, restore, build, test; fresh nonzero passing TRX; independent review; exact manifest enforcement; preservation of verified tests. Read inputs completely; source mismatch is a blocker, not permission to alter predecessors. Write higher-token-budget files first; after all four writes the host runs actual checks without another source-heavy Developer request. All real model calls enforce context budgets. This slice establishes immutable evidence fidelity only, not DEX integration or arbitrage acceptance.
