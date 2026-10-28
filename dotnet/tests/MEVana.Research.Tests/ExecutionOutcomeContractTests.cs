using System.Globalization;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

internal sealed record OutcomeInput
{
    internal string? Id { get; init; } = "synthetic-rejected";
    internal int? Schema { get; init; } = 1;
    internal string? Key { get; init; } = "synthetic-idempotency-rejected";
    internal string? Dataset { get; init; } = "synthetic-dataset";
    internal string? Metadata { get; init; } = "synthetic-metadata-v1";
    internal string? Strategy { get; init; } = "synthetic-strategy-v1";
    internal PolicyKind? Policy { get; init; } = PolicyKind.SimpleRules;
    internal string? Ranking { get; init; }
    internal int? Rank { get; init; }
    internal string? Experiment { get; init; } = "synthetic-result";
    internal ExecutionMode? Mode { get; init; } = ExecutionMode.Shadow;
    internal EvidenceKind? Evidence { get; init; } = EvidenceKind.Synthetic;
    internal ResearchRoute? Route { get; init; } = Stage0Fixtures.Route();
    internal long? Size { get; init; } = 10;
    internal long? Observed { get; init; } = 10;
    internal DateTimeOffset? Detected { get; init; } = Stage0Fixtures.Start.AddDays(2);
    internal DateTimeOffset? Decision { get; init; } = Stage0Fixtures.Start.AddDays(2).AddSeconds(1);
    internal DateTimeOffset? Submitted { get; init; }
    internal long? Landed { get; init; }
    internal DateTimeOffset? Reconciled { get; init; }
    internal ExecutionOutcomeState? State { get; init; } = ExecutionOutcomeState.Rejected;
    internal string? Reason { get; init; } = "synthetic-size-filter";
    internal string? Signature { get; init; }
    internal long? Estimated { get; init; } = -5;
    internal long? Simulated { get; init; }
    internal long? Realized { get; init; }
    internal ulong? Network { get; init; }
    internal ulong? Priority { get; init; }
    internal ulong? Tip { get; init; }
    internal ExecutionOutcome Build() => new(Id, Schema, Key, Dataset, Metadata, Strategy, Policy, Ranking, Rank, Experiment,
        Mode, Evidence, Route, Size, Observed, Detected, Decision, Submitted, Landed, Reconciled, State, Reason, Signature,
        Estimated, Simulated, Realized, Network, Priority, Tip);
    internal static OutcomeInput ForState(ExecutionOutcomeState state, ExecutionMode mode = ExecutionMode.Paper, EvidenceKind evidence = EvidenceKind.Synthetic)
    {
        bool submitted = state is ExecutionOutcomeState.Landed or ExecutionOutcomeState.Failed or ExecutionOutcomeState.Dropped;
        return new() { Mode = mode, Evidence = evidence, State = state,
            Submitted = submitted ? Stage0Fixtures.Start.AddDays(2).AddSeconds(2) : null,
            Landed = state is ExecutionOutcomeState.Landed or ExecutionOutcomeState.Failed ? 11 : null,
            Signature = submitted && mode == ExecutionMode.LiveSubmission ? new string('1', 64) : null,
            Reason = state is ExecutionOutcomeState.Landed or ExecutionOutcomeState.NotSubmitted ? null : "synthetic-reason" };
    }
    internal static OutcomeInput PaperLanded() => ForState(ExecutionOutcomeState.Landed) with {
        Id = "synthetic-paper-landed", Key = "synthetic-idempotency-paper-landed", Policy = PolicyKind.ModelRanking,
        Ranking = "synthetic-ranking", Rank = 1, Reconciled = Stage0Fixtures.Start.AddDays(2).AddSeconds(3), Network = 3, Priority = 2, Tip = 0 };
}

public class ExecutionOutcomeContractTests
{
    static IReadOnlyList<ValidationIssue> Syntax(OutcomeInput o) => ExecutionOutcomeSyntaxValidator.Validate(o.Build());
    static string Canon(OutcomeInput o) => ExecutionOutcomeCanonicalizer.Canonicalize(o.Build()).Content;
    static void Bad(OutcomeInput o, string code) => Assert.Contains(Syntax(o), e => e.Code == code);

    [Fact] public void IntegerOnlyOutcomeSurfaceHasFactsWithoutTradingAuthority()
    {
        Stage0Fixtures.Surface<ExecutionOutcome>("OutcomeId", "SchemaVersion", "IdempotencyKey", "DatasetId", "DatasetMetadataVersion", "StrategyVersion", "PolicyKind",
            "RankingArtifactId", "RankingEntryRank", "ExperimentResultId", "Mode", "EvidenceKind", "Route", "InputSizeQuoteAtoms", "ObservedSlot", "DetectedAtUtc", "DecisionAtUtc", "SubmittedAtUtc",
            "LandedSlot", "ReconciledAtUtc", "State", "ReasonCode", "TransactionSignature", "EstimatedNetProfitQuoteAtoms", "SimulatedNetProfitQuoteAtoms", "RealizedNetProfitQuoteAtoms",
            "NetworkFeeQuoteAtoms", "PriorityFeeQuoteAtoms", "TipQuoteAtoms");
        Stage0Fixtures.IntegerSurface(typeof(ExecutionOutcome), typeof(ResearchRoute));
        foreach (var name in new[] { "NetworkFeeQuoteAtoms", "PriorityFeeQuoteAtoms", "TipQuoteAtoms" }) Assert.Equal(typeof(ulong?), typeof(ExecutionOutcome).GetProperty(name)!.PropertyType);
        Assert.Equal(typeof(ResearchRoute), Nullable.GetUnderlyingType(typeof(ExecutionOutcome).GetProperty("Route")!.PropertyType) ?? typeof(ExecutionOutcome).GetProperty("Route")!.PropertyType);
        Assert.Equal(new[] { "Shadow", "Paper", "LiveSubmission" }, Enum.GetNames<ExecutionMode>());
        Assert.Equal(new[] { "Rejected", "NotSubmitted", "Landed", "Failed", "Dropped", "Expired" }, Enum.GetNames<ExecutionOutcomeState>());
        using var json = JsonDocument.Parse(Canon(new()));
        Assert.Equal(new[] { "canonicalizationVersion", "schemaVersion", "outcomeId", "idempotencyKey", "datasetId", "datasetMetadataVersion", "strategyVersion", "policyKind",
            "rankingArtifactId", "rankingEntryRank", "experimentResultId", "mode", "evidenceKind", "route", "inputSizeQuoteAtoms", "observedSlot", "detectedAtUtc", "decisionAtUtc", "submittedAtUtc",
            "landedSlot", "reconciledAtUtc", "state", "reasonCode", "transactionSignature", "estimatedNetProfitQuoteAtoms", "simulatedNetProfitQuoteAtoms", "realizedNetProfitQuoteAtoms",
            "networkFeeQuoteAtoms", "priorityFeeQuoteAtoms", "tipQuoteAtoms" }, json.RootElement.EnumerateObject().Select(p => p.Name));
        Assert.Equal(new[] { "tokenIds", "poolIds" }, json.RootElement.GetProperty("route").EnumerateObject().Select(p => p.Name));
    }
    [Fact] public void RawInvalidScalarsAndSharedRouteSnapshotRemainInspectable()
    {
        var tokens = new List<string?> { "", null, "" }; var pools = new List<string?> { null, null };
        var route = new ResearchRoute(tokens, pools); var offset = Stage0Fixtures.Start.ToOffset(TimeSpan.FromHours(-6));
        var o = new OutcomeInput { Id = "", Key = "bad/key", Schema = -1, Mode = (ExecutionMode)99, Evidence = (EvidenceKind)99,
            State = (ExecutionOutcomeState)99, Policy = (PolicyKind)99, Route = route, Size = -1, Observed = -2, Rank = -3,
            Detected = offset, Decision = offset, Submitted = offset, Reconciled = offset, Landed = -4, Signature = "BAD", Estimated = long.MinValue, Simulated = long.MaxValue, Realized = -5, Network = ulong.MaxValue }.Build();
        tokens.Clear(); pools.Clear();
        Assert.Same(route, o.Route); Assert.Equal(new string?[] { "", null, "" }, o.Route!.TokenIds);
        Assert.Equal("", o.OutcomeId); Assert.Equal("bad/key", o.IdempotencyKey); Assert.Equal(-1, o.SchemaVersion);
        Assert.Equal((ExecutionMode)99, o.Mode); Assert.Equal((EvidenceKind)99, o.EvidenceKind); Assert.Equal((ExecutionOutcomeState)99, o.State); Assert.Equal((PolicyKind)99, o.PolicyKind);
        Assert.Equal(-1, o.InputSizeQuoteAtoms); Assert.Equal(-2, o.ObservedSlot); Assert.Equal(-3, o.RankingEntryRank); Assert.Equal(-4, o.LandedSlot);
        foreach (var time in new[] { o.DetectedAtUtc, o.DecisionAtUtc, o.SubmittedAtUtc, o.ReconciledAtUtc }) Assert.Equal(offset.Offset, time!.Value.Offset);
        Assert.Equal("BAD", o.TransactionSignature); Assert.Equal(long.MinValue, o.EstimatedNetProfitQuoteAtoms); Assert.Equal(long.MaxValue, o.SimulatedNetProfitQuoteAtoms); Assert.Equal(-5, o.RealizedNetProfitQuoteAtoms);
        Assert.Equal(ulong.MaxValue, o.NetworkFeeQuoteAtoms); Assert.Throws<NotSupportedException>(() => ((IList<string?>)o.Route.TokenIds!).Clear());
        var empty = new ExecutionOutcome(null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null);
        Assert.All(typeof(ExecutionOutcome).GetProperties(), p => Assert.Null(p.GetValue(empty)));
    }
    [Theory] [InlineData(null, "required")] [InlineData("", "invalid_identifier")] [InlineData("é", "invalid_identifier")]
    [InlineData("a/b", "invalid_identifier")] [InlineData("a.b", "invalid_identifier")]
    public void IdentityAndOptionalIdentifiers(string? id, string code)
    {
        var o = new OutcomeInput();
        foreach (var changed in new[] { o with { Id = id }, o with { Key = id }, o with { Dataset = id }, o with { Metadata = id }, o with { Strategy = id }, o with { Reason = id } }) Bad(changed, code);
        if (id is not null) { Bad(o with { Ranking = id, Rank = 1 }, code); Bad(o with { Experiment = id }, code); }
        Assert.Empty(Syntax(o with { Id = new string('a', 64), Key = "key-_09", Experiment = null }));
        Bad(o with { Key = new string('a', 65) }, "invalid_identifier");
    }
    [Theory] [InlineData(PolicyKind.FixedUniverse)] [InlineData(PolicyKind.SimpleRules)] [InlineData(PolicyKind.ModelRanking)]
    public void RankingLinkIsPairedAndUnrankedCoverageIsRepresentable(PolicyKind policy)
    {
        var o = new OutcomeInput { Policy = policy };
        Assert.Empty(Syntax(o)); Assert.Empty(Syntax(o with { Ranking = "ranking", Rank = 1 })); Assert.Empty(Syntax(o with { Ranking = "ranking", Rank = 1000 }));
        Bad(o with { Ranking = "ranking" }, "invalid_ranking_link"); Bad(o with { Rank = 1 }, "invalid_ranking_link");
        Bad(o with { Ranking = "ranking", Rank = 0 }, "invalid_rank"); Bad(o with { Ranking = "ranking", Rank = 1001 }, "invalid_rank");
    }
    [Fact] public void SchemaAndAllEnumsAreRequiredAndDefined()
    {
        var o = new OutcomeInput();
        foreach (var changed in new[] { o with { Schema = null }, o with { Policy = null }, o with { Evidence = null }, o with { Mode = null }, o with { State = null } }) Bad(changed, "required");
        Bad(o with { Schema = 2 }, "unsupported_schema_version");
        foreach (var changed in new[] { o with { Policy = (PolicyKind)99 }, o with { Evidence = (EvidenceKind)99 }, o with { Mode = (ExecutionMode)99 }, o with { State = (ExecutionOutcomeState)99 } }) Bad(changed, "invalid_enum");
    }
    public static IEnumerable<object[]> StateMatrix()
    {
        foreach (var mode in Enum.GetValues<ExecutionMode>())
        foreach (var evidence in Enum.GetValues<EvidenceKind>())
        foreach (var state in Enum.GetValues<ExecutionOutcomeState>()) yield return [mode, evidence, state];
    }
    [Theory] [MemberData(nameof(StateMatrix))]
    public void CompleteModeEvidenceTerminalMatrix(ExecutionMode mode, EvidenceKind evidence, ExecutionOutcomeState state)
    {
        var o = OutcomeInput.ForState(state, mode, evidence);
        bool submitted = state is ExecutionOutcomeState.Landed or ExecutionOutcomeState.Failed or ExecutionOutcomeState.Dropped;
        bool compatibleEvidence = (mode == ExecutionMode.LiveSubmission) == (evidence == EvidenceKind.Live);
        bool valid = compatibleEvidence && (!submitted || mode != ExecutionMode.Shadow);
        if (valid) Assert.Empty(Syntax(o));
        else Assert.NotEmpty(Syntax(o));
        if (!compatibleEvidence) Bad(o, "incompatible_evidence");
        if (submitted && mode == ExecutionMode.Shadow) Bad(o, "submission_not_allowed");
    }
    [Theory] [InlineData(ExecutionOutcomeState.Rejected)] [InlineData(ExecutionOutcomeState.Failed)] [InlineData(ExecutionOutcomeState.Dropped)] [InlineData(ExecutionOutcomeState.Expired)]
    public void RequiredTerminalReasons(ExecutionOutcomeState state)
    {
        var o = OutcomeInput.ForState(state); Bad(o with { Reason = null }, "required"); Bad(o with { Reason = "" }, "invalid_identifier");
    }
    [Theory] [InlineData(ExecutionOutcomeState.Landed)] [InlineData(ExecutionOutcomeState.Failed)] [InlineData(ExecutionOutcomeState.Dropped)]
    public void SubmittedStatesRequireTimeAndLiveSignature(ExecutionOutcomeState state)
    {
        var paper = OutcomeInput.ForState(state); Bad(paper with { Submitted = null }, "required");
        var live = OutcomeInput.ForState(state, ExecutionMode.LiveSubmission, EvidenceKind.Live);
        Bad(live with { Signature = null }, "required"); Bad(live with { Signature = "" }, "invalid_signature");
        Assert.Empty(Syntax(live));
    }
    [Theory] [InlineData(64)] [InlineData(88)]
    public void SignatureBase58LengthBoundariesAreSyntaxOnly(int length)
    {
        const string alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        var signature = new string(Enumerable.Range(0, length).Select(i => alphabet[i % alphabet.Length]).ToArray());
        Assert.Empty(Syntax(OutcomeInput.ForState(ExecutionOutcomeState.Landed, ExecutionMode.LiveSubmission, EvidenceKind.Live) with { Signature = signature }));
    }
    [Theory] [InlineData('0')] [InlineData('O')] [InlineData('I')] [InlineData('l')] [InlineData('+')] [InlineData('/')] [InlineData('=')] [InlineData('é')] [InlineData(' ')]
    public void NonBase58SignatureCharactersAreRejected(char bad)
    {
        Bad(OutcomeInput.ForState(ExecutionOutcomeState.Failed, ExecutionMode.LiveSubmission, EvidenceKind.Live) with { Signature = new string('1', 63) + bad }, "invalid_signature");
    }
    [Fact] public void SignatureLengthsAndForbiddenContexts()
    {
        var live = OutcomeInput.ForState(ExecutionOutcomeState.Landed, ExecutionMode.LiveSubmission, EvidenceKind.Live);
        foreach (var length in new[] { 63, 89 }) Bad(live with { Signature = new string('1', length) }, "invalid_signature");
        Bad(OutcomeInput.PaperLanded() with { Signature = new string('1', 64) }, "signature_not_allowed");
        Bad(new OutcomeInput { Signature = new string('1', 64) }, "signature_not_allowed");
    }
    [Fact] public void LandedSlotsRequiredOnlyForLandedAndFailed()
    {
        foreach (var state in new[] { ExecutionOutcomeState.Landed, ExecutionOutcomeState.Failed })
        {
            var o = OutcomeInput.ForState(state); Bad(o with { Landed = null }, "required"); Bad(o with { Landed = -1 }, "invalid_slot"); Bad(o with { Landed = 9 }, "invalid_slot");
            Assert.Empty(Syntax(o with { Landed = 10 })); Assert.Empty(Syntax(o with { Observed = long.MaxValue, Landed = long.MaxValue }));
        }
        foreach (var state in new[] { ExecutionOutcomeState.Rejected, ExecutionOutcomeState.NotSubmitted, ExecutionOutcomeState.Dropped, ExecutionOutcomeState.Expired })
            Bad(OutcomeInput.ForState(state) with { Landed = 10 }, "invalid_slot");
    }
    [Fact] public void CandidateFactsRequiredPositiveAndNonnegative()
    {
        var o = new OutcomeInput();
        foreach (var changed in new[] { o with { Route = null }, o with { Size = null }, o with { Observed = null }, o with { Detected = null }, o with { Decision = null } }) Bad(changed, "required");
        Bad(o with { Size = 0 }, "invalid_size"); Bad(o with { Size = -1 }, "invalid_size"); Bad(o with { Observed = -1 }, "invalid_count");
        Assert.Empty(Syntax(o with { Size = long.MaxValue, Observed = 0 }));
        Bad(o with { Route = new(["A", "B", "A"], ["P", "P"]) }, "invalid_route");
    }
    [Fact] public void EveryTimeIsUtcAndPresentTimesAreMonotonic()
    {
        var o = OutcomeInput.PaperLanded(); var offset = o.Detected!.Value.ToOffset(TimeSpan.FromHours(-6));
        foreach (var changed in new[] { o with { Detected = offset }, o with { Decision = offset }, o with { Submitted = offset }, o with { Reconciled = offset } }) Bad(changed, "invalid_utc");
        Bad(o with { Decision = o.Detected!.Value.AddTicks(-1) }, "non_monotonic_time");
        Bad(o with { Submitted = o.Decision!.Value.AddTicks(-1) }, "non_monotonic_time");
        Bad(o with { Reconciled = o.Submitted!.Value.AddTicks(-1) }, "non_monotonic_time");
        Bad(new OutcomeInput { Reconciled = Stage0Fixtures.Start }, "non_monotonic_time");
        Assert.Empty(Syntax(o with { Decision = o.Detected, Submitted = o.Detected, Reconciled = o.Detected }));
        foreach (var state in new[] { ExecutionOutcomeState.Rejected, ExecutionOutcomeState.NotSubmitted, ExecutionOutcomeState.Expired })
            Bad(OutcomeInput.ForState(state) with { Submitted = o.Submitted }, "submission_not_allowed");
    }
    [Fact] public void ProfitRulesAcrossEvidenceAndStates()
    {
        foreach (var evidence in Enum.GetValues<EvidenceKind>())
        foreach (var state in Enum.GetValues<ExecutionOutcomeState>())
        {
            var mode = evidence == EvidenceKind.Live ? ExecutionMode.LiveSubmission : ExecutionMode.Paper;
            var o = OutcomeInput.ForState(state, mode, evidence) with { Reconciled = Stage0Fixtures.Start.AddDays(2).AddSeconds(3) };
            bool realized = mode == ExecutionMode.LiveSubmission && state is ExecutionOutcomeState.Landed or ExecutionOutcomeState.Failed;
            if (realized) Assert.Empty(Syntax(o with { Realized = -1 })); else Bad(o with { Realized = 0 }, "profit_not_allowed");
            if (evidence == EvidenceKind.Synthetic) Bad(o with { Simulated = 0 }, "profit_not_allowed"); else Assert.Empty(Syntax(o with { Simulated = long.MinValue }));
        }
        var live = OutcomeInput.ForState(ExecutionOutcomeState.Landed, ExecutionMode.LiveSubmission, EvidenceKind.Live);
        Bad(live with { Realized = 1 }, "required");
    }
    [Fact] public void FailedLiveLossesAndUnsignedCostsRetainExactIndependentValues()
    {
        var o = OutcomeInput.ForState(ExecutionOutcomeState.Failed, ExecutionMode.LiveSubmission, EvidenceKind.Live) with {
            Reconciled = Stage0Fixtures.Start.AddDays(2).AddSeconds(3), Estimated = long.MaxValue, Simulated = long.MinValue, Realized = -7,
            Network = ulong.MaxValue, Priority = ulong.MaxValue, Tip = 0 };
        Assert.Empty(Syntax(o)); using var json = JsonDocument.Parse(Canon(o)); var root = json.RootElement;
        Assert.Equal(long.MaxValue, root.GetProperty("estimatedNetProfitQuoteAtoms").GetInt64()); Assert.Equal(long.MinValue, root.GetProperty("simulatedNetProfitQuoteAtoms").GetInt64());
        Assert.Equal(-7, root.GetProperty("realizedNetProfitQuoteAtoms").GetInt64()); Assert.Equal(ulong.MaxValue, root.GetProperty("networkFeeQuoteAtoms").GetUInt64());
        Assert.Equal(ulong.MaxValue, root.GetProperty("priorityFeeQuoteAtoms").GetUInt64()); Assert.Equal(0UL, root.GetProperty("tipQuoteAtoms").GetUInt64());
    }
    [Fact] public void ProfitNeverFallsBackAcrossChannelsOrHelperCalls()
    {
        var o = OutcomeInput.ForState(ExecutionOutcomeState.Landed, ExecutionMode.LiveSubmission, EvidenceKind.Live) with { Reconciled = Stage0Fixtures.Start.AddDays(2).AddSeconds(3) };
        foreach (var changed in new[] { o with { Estimated = long.MinValue }, o with { Estimated = null, Simulated = long.MaxValue }, o with { Estimated = null, Realized = -1 }, o with { Estimated = null } })
        {
            var raw = changed.Build(); Assert.False(ExecutionOutcomeIdempotency.HasConflict(raw, changed.Build()));
            using var json = JsonDocument.Parse(Canon(changed)); var root = json.RootElement;
            Assert.Equal(changed.Estimated, root.GetProperty("estimatedNetProfitQuoteAtoms").Deserialize<long?>());
            Assert.Equal(changed.Simulated, root.GetProperty("simulatedNetProfitQuoteAtoms").Deserialize<long?>());
            Assert.Equal(changed.Realized, root.GetProperty("realizedNetProfitQuoteAtoms").Deserialize<long?>());
            Assert.Equal(changed.Realized, raw.RealizedNetProfitQuoteAtoms);
        }
    }
    [Fact] public void CostsForbiddenWhenNotSubmittedEvenIfZero()
    {
        foreach (var state in new[] { ExecutionOutcomeState.Rejected, ExecutionOutcomeState.NotSubmitted, ExecutionOutcomeState.Expired })
        {
            var o = OutcomeInput.ForState(state); Bad(o with { Network = 0 }, "costs_not_allowed"); Bad(o with { Priority = 0 }, "costs_not_allowed"); Bad(o with { Tip = 0 }, "costs_not_allowed");
        }
        foreach (var state in new[] { ExecutionOutcomeState.Landed, ExecutionOutcomeState.Failed, ExecutionOutcomeState.Dropped })
            Assert.Empty(Syntax(OutcomeInput.ForState(state) with { Network = 0, Priority = 0, Tip = 0 }));
    }
    [Fact] public void CatalogProvenanceCoverageRouteAndFirstQuoteBound()
    {
        var c = Stage0Fixtures.Catalog(); var o = new OutcomeInput();
        void Check(OutcomeInput input, string code) => Assert.Contains(ExecutionOutcomeValidator.Validate(input.Build(), c), e => e.Code == code);
        Assert.Empty(ExecutionOutcomeValidator.Validate(o.Build(), c)); Assert.Empty(ExecutionOutcomeValidator.Validate(o.Build(), Stage0Fixtures.Catalog(false)));
        Check(o with { Dataset = "Synthetic-dataset" }, "unknown_dataset"); Check(o with { Metadata = "other" }, "metadata_version_mismatch"); Check(o with { Strategy = "other" }, "unsupported_strategy");
        Check(o with { Detected = Stage0Fixtures.Start.AddTicks(-1) }, "invalid_window");
        Check(o with { Detected = Stage0Fixtures.Start.AddDays(11), Decision = Stage0Fixtures.Start.AddDays(11) }, "invalid_window");
        Check(o with { Route = new(["unknown", Stage0Fixtures.B, "unknown"], [Stage0Fixtures.P, Stage0Fixtures.Q]) }, "unknown_token");
        Check(o with { Route = new([Stage0Fixtures.A, Stage0Fixtures.B, Stage0Fixtures.A], ["unknown", Stage0Fixtures.Q]) }, "unknown_pool");
        Check(o with { Route = new([Stage0Fixtures.A, Stage0Fixtures.C, Stage0Fixtures.A], [Stage0Fixtures.P, Stage0Fixtures.Q]) }, "incompatible_pool");
        Check(o with { Size = 11 }, "invalid_trade_size"); Assert.Empty(ExecutionOutcomeValidator.Validate((o with { Size = 11 }).Build(), Stage0Fixtures.Catalog(quoteBound: 11)));
        Assert.Contains(ExecutionOutcomeValidator.Validate(o.Build(), Stage0Fixtures.Catalog(quoteBound: 100, datasetBound: 9)), e => e.Code == "invalid_trade_size");
        Assert.Empty(ExecutionOutcomeValidator.Validate((o with { Detected = Stage0Fixtures.Start, Decision = Stage0Fixtures.Start.AddDays(20) }).Build(), c));
        Assert.Empty(ExecutionOutcomeValidator.Validate((o with { Detected = Stage0Fixtures.Start.AddDays(10), Decision = Stage0Fixtures.Start.AddDays(10) }).Build(), c));
    }
    [Fact] public void SyntaxBeforeLookupImmutableIssuesAndInvalidCanonicalization()
    {
        var o = new OutcomeInput { Id = null }; var errors = Syntax(o);
        Assert.Equal(errors, ExecutionOutcomeValidator.Validate(o.Build(), ResearchCatalog.Empty)); Assert.Equal(errors, Syntax(o));
        Assert.Throws<NotSupportedException>(() => ((IList<ValidationIssue>)errors).Clear());
        Assert.Throws<ArgumentNullException>(() => ExecutionOutcomeValidator.Validate(o.Build(), null!));
        Assert.Equal("required", Assert.Single(ExecutionOutcomeSyntaxValidator.Validate(null)).Code);
        Assert.Throws<ArgumentNullException>(() => ExecutionOutcomeCanonicalizer.Canonicalize(null!)); Assert.Throws<ArgumentException>(() => Canon(o));
        Assert.Throws<ArgumentException>(() => ExecutionOutcomeIdempotency.HasConflict(o.Build(), new OutcomeInput().Build()));
        Assert.Throws<ArgumentException>(() => ExecutionOutcomeIdempotency.HasConflict(new OutcomeInput().Build(), o.Build()));
        Assert.Throws<ArgumentNullException>(() => ExecutionOutcomeIdempotency.HasConflict(null!, new OutcomeInput().Build()));
        Assert.Throws<ArgumentNullException>(() => ExecutionOutcomeIdempotency.HasConflict(new OutcomeInput().Build(), null!));
    }
    [Fact] public void IdempotencyConflictUsesExactContentAndOrdinalKeyWithoutMutation()
    {
        var o = new OutcomeInput(); var raw = o.Build(); var before = ExecutionOutcomeCanonicalizer.Canonicalize(raw);
        Assert.False(ExecutionOutcomeIdempotency.HasConflict(raw, o.Build()));
        foreach (var changed in new[] { o with { Id = "other" }, o with { Estimated = -6 }, o with { Route = Stage0Fixtures.Route(true) }, o with { Policy = PolicyKind.FixedUniverse } })
            Assert.True(ExecutionOutcomeIdempotency.HasConflict(raw, changed.Build()));
        Assert.False(ExecutionOutcomeIdempotency.HasConflict(raw, (o with { Key = "different" }).Build()));
        Assert.False(ExecutionOutcomeIdempotency.HasConflict(raw, (o with { Key = "Synthetic-idempotency-rejected" }).Build()));
        Assert.Equal(before, ExecutionOutcomeCanonicalizer.Canonicalize(raw));
    }
    [Fact] public void CanonicalCultureNullChannelsRouteOrderAndSyntheticGoldenCases()
    {
        var rejected = new OutcomeInput(); var landed = OutcomeInput.PaperLanded(); var culture = CultureInfo.CurrentCulture;
        var r = Canon(rejected); var l = Canon(landed);
        try { CultureInfo.CurrentCulture = new("tr-TR"); Assert.Equal(r, Canon(rejected)); Assert.Equal(l, Canon(landed)); }
        finally { CultureInfo.CurrentCulture = culture; }
        Stage0Fixtures.Golden("2026-10-28-synthetic-rejected-outcome-v1.json", r);
        Stage0Fixtures.Golden("2026-10-28-synthetic-paper-landed-outcome-v1.json", l);
        Assert.Equal(1, ExecutionOutcomeCanonicalizer.Canonicalize(landed.Build()).Version);
        Assert.NotEqual(r, Canon(rejected with { Route = Stage0Fixtures.Route(true) }));
        foreach (var canonical in new[] { r, l })
        {
            using var json = JsonDocument.Parse(canonical); var root = json.RootElement;
            Assert.Equal(JsonValueKind.Null, root.GetProperty("simulatedNetProfitQuoteAtoms").ValueKind); Assert.Equal(JsonValueKind.Null, root.GetProperty("realizedNetProfitQuoteAtoms").ValueKind);
            Assert.Equal("2026-01-03T00:00:00.0000000Z", root.GetProperty("detectedAtUtc").GetString());
            foreach (var name in new[] { "outcomeId", "idempotencyKey", "datasetId", "datasetMetadataVersion", "strategyVersion", "experimentResultId" }) Assert.StartsWith("synthetic-", root.GetProperty(name).GetString());
        }
        using var paper = JsonDocument.Parse(l); Assert.Equal("Paper", paper.RootElement.GetProperty("mode").GetString()); Assert.Equal("Landed", paper.RootElement.GetProperty("state").GetString());
        using var shadow = JsonDocument.Parse(r); Assert.Equal("Rejected", shadow.RootElement.GetProperty("state").GetString());
    }
}
