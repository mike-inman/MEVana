using System.Globalization;
using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

internal static class Stage0Fixtures
{
    internal const string A = "synthetic-A", B = "synthetic-B", C = "synthetic-C", D = "synthetic-D";
    internal const string P = "synthetic-P", Q = "synthetic-Q", R = "synthetic-R", S = "synthetic-S", T = "synthetic-T";
    internal static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-01-01T00:00:00Z", CultureInfo.InvariantCulture);
    internal static ResearchRoute Route(bool reverse = false) => new([A, B, A], reverse ? [Q, P] : [P, Q]);
    internal static ResearchCatalog Catalog(bool available = true, long quoteBound = 10, long datasetBound = 50) => new([new(
        "synthetic-dataset", "synthetic-metadata-v1", "Synthetic contract data", available, "Tests only", Start, Start.AddDays(10), ["synthetic-strategy-v1"],
        [new(A, 6, quoteBound), new(B, 9, 100), new(C, 6, 100), new(D, 6, 100)],
        [new(P, [A, B]), new(Q, [B, A]), new(R, [B, C]), new(S, [C, D]), new(T, [D, A])], datasetBound)]);
    internal static void Surface<T>(params string[] expected) => Assert.Equal(expected.Order(), typeof(T).GetProperties().Select(p => p.Name).Order());
    internal static void IntegerSurface(params Type[] types)
    {
        foreach (var type in types)
        foreach (var p in type.GetProperties())
        {
            var t = Nullable.GetUnderlyingType(p.PropertyType) ?? p.PropertyType;
            Assert.DoesNotContain(t, new[] { typeof(float), typeof(double), typeof(decimal) });
        }
    }
    internal static void Golden(string filename, string canonical)
    {
        var bytes = File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", filename));
        Assert.Equal(bytes, Encoding.UTF8.GetBytes(canonical));
        using var doc = JsonDocument.Parse(bytes);
        Assert.Equal("Synthetic", doc.RootElement.GetProperty("evidenceKind").GetString());
    }
}

internal sealed record RankingEntryInput
{
    internal int? Rank { get; init; } = 1;
    internal ResearchRoute? Route { get; init; } = Stage0Fixtures.Route();
    internal long? Min { get; init; } = 1;
    internal long? Max { get; init; } = 10;
    internal int? Priority { get; init; } = 90;
    internal int? Confidence { get; init; } = 8200;
    internal int? Probability { get; init; } = 6100;
    internal long? Samples { get; init; } = 0;
    internal long? Value { get; init; } = -5;
    internal RankingEntry Build() => new(Rank, Route, Min, Max, Priority, Confidence, Probability, Samples, Value);
}
internal sealed record RankingInput
{
    internal string? Id { get; init; } = "synthetic-ranking";
    internal int? Schema { get; init; } = 1;
    internal PolicyKind? Policy { get; init; } = PolicyKind.ModelRanking;
    internal EvidenceKind? Evidence { get; init; } = EvidenceKind.Synthetic;
    internal string? Strategy { get; init; } = "synthetic-strategy-v1";
    internal string? Features { get; init; } = "synthetic-features-v1";
    internal string? Model { get; init; } = "synthetic-model-v1";
    internal string? Dataset { get; init; } = "synthetic-dataset";
    internal string? Metadata { get; init; } = "synthetic-metadata-v1";
    internal DateTimeOffset? TrainingStart { get; init; } = Stage0Fixtures.Start;
    internal DateTimeOffset? TrainingEnd { get; init; } = Stage0Fixtures.Start.AddDays(2);
    internal string? Source { get; init; } = "synthetic-result";
    internal DateTimeOffset? AsOf { get; init; } = Stage0Fixtures.Start.AddDays(2);
    internal long? Slot { get; init; } = 0;
    internal DateTimeOffset? From { get; init; } = Stage0Fixtures.Start.AddDays(2);
    internal DateTimeOffset? Expiry { get; init; } = Stage0Fixtures.Start.AddDays(3);
    internal IReadOnlyList<RankingEntry?>? Entries { get; init; } = [new RankingEntryInput().Build()];
    internal RankingArtifact Build() => new(Id, Schema, Policy, Evidence, Strategy, Features, Model, Dataset, Metadata,
        TrainingStart, TrainingEnd, Source, AsOf, Slot, From, Expiry, Entries);
}

public class RankingArtifactContractTests
{
    static IReadOnlyList<ValidationIssue> Syntax(RankingInput a) => RankingArtifactSyntaxValidator.Validate(a.Build());
    static string Canon(RankingInput a) => RankingArtifactCanonicalizer.Canonicalize(a.Build()).Content;
    static void Bad(RankingInput a, string code) => Assert.Contains(Syntax(a), i => i.Code == code);
    static RankingInput Entry(RankingEntryInput e) => new() { Entries = [e.Build()] };

    [Fact] public void AdvisoryIntegerOnlySurfaceHasNoTradingAuthorityOrProfitChannels()
    {
        Stage0Fixtures.Surface<RankingArtifact>("ArtifactId", "SchemaVersion", "Kind", "PolicyKind", "EvidenceKind", "StrategyVersion", "FeatureSetVersion", "ModelVersion",
            "DatasetId", "DatasetMetadataVersion", "TrainingStartUtc", "TrainingEndUtc", "SourceExperimentResultId", "AsOfUtc", "ObservedSlot", "ValidFromUtc", "ExpiresAtUtc", "Entries");
        Stage0Fixtures.Surface<RankingEntry>("Rank", "Route", "MinSizeQuoteAtoms", "MaxSizeQuoteAtoms", "MonitoringPriority", "ResearchConfidenceBps", "CalibratedSuccessProbabilityBps", "SampleCount", "EstimatedNetValueQuoteAtoms");
        Stage0Fixtures.Surface<ResearchRoute>("TokenIds", "PoolIds");
        Stage0Fixtures.IntegerSurface(typeof(RankingArtifact), typeof(RankingEntry), typeof(ResearchRoute));
        using var json = JsonDocument.Parse(Canon(new()));
        Assert.Equal("WATCH", json.RootElement.GetProperty("kind").GetString());
        Assert.Equal(new[] { "canonicalizationVersion", "schemaVersion", "kind", "artifactId", "policyKind", "evidenceKind", "strategyVersion", "featureSetVersion", "modelVersion", "datasetId", "datasetMetadataVersion",
            "trainingStartUtc", "trainingEndUtc", "sourceExperimentResultId", "asOfUtc", "observedSlot", "validFromUtc", "expiresAtUtc", "entries" }, json.RootElement.EnumerateObject().Select(p => p.Name));
        Assert.Equal(new[] { "rank", "route", "minSizeQuoteAtoms", "maxSizeQuoteAtoms", "monitoringPriority", "researchConfidenceBps", "calibratedSuccessProbabilityBps", "sampleCount", "estimatedNetValueQuoteAtoms" },
            json.RootElement.GetProperty("entries")[0].EnumerateObject().Select(p => p.Name));
    }
    [Fact] public void RawValuesNullsOffsetsDuplicatesAndListsArePreserved()
    {
        var tokens = new List<string?> { "", null, "" }; var pools = new List<string?> { null, null };
        var route = new ResearchRoute(tokens, pools); tokens.Clear(); pools.Clear();
        Assert.Equal(new string?[] { "", null, "" }, route.TokenIds); Assert.Equal(new string?[] { null, null }, route.PoolIds);
        var entry = new RankingEntry(null, route, -1, null, 999, -2, 12345, -1, long.MinValue);
        var entries = new List<RankingEntry?> { entry, null, entry };
        var offset = Stage0Fixtures.Start.ToOffset(TimeSpan.FromHours(-5));
        var a = new RankingInput { Id = "", Schema = -1, Model = "bad/path", Policy = (PolicyKind)999, Evidence = (EvidenceKind)999, TrainingStart = offset, Entries = entries }.Build();
        entries.Clear(); Assert.Equal(new RankingEntry?[] { entry, null, entry }, a.Entries);
        Assert.Equal(offset.Offset, a.TrainingStartUtc!.Value.Offset); Assert.Equal("", a.ArtifactId); Assert.Equal(-1, a.SchemaVersion);
        Assert.Equal((PolicyKind)999, a.PolicyKind); Assert.Equal((EvidenceKind)999, a.EvidenceKind); Assert.Equal("bad/path", a.ModelVersion);
        Assert.Equal(-1, entry.MinSizeQuoteAtoms); Assert.Null(entry.MaxSizeQuoteAtoms); Assert.Equal(999, entry.MonitoringPriority);
        Assert.Equal(-2, entry.ResearchConfidenceBps); Assert.Equal(12345, entry.CalibratedSuccessProbabilityBps); Assert.Equal(-1, entry.SampleCount); Assert.Equal(long.MinValue, entry.EstimatedNetValueQuoteAtoms);
        Assert.Throws<NotSupportedException>(() => ((IList<string?>)route.TokenIds!).Clear());
        Assert.Throws<NotSupportedException>(() => ((IList<string?>)route.PoolIds!).Clear());
        Assert.Throws<NotSupportedException>(() => ((IList<RankingEntry?>)a.Entries!).Clear());
        var allNull = new RankingArtifact(null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null);
        Assert.Null(allNull.Entries); Assert.Null(allNull.ArtifactId); Assert.Null(allNull.ObservedSlot); Assert.Null(new ResearchRoute(null, null).TokenIds);
    }
    [Theory] [InlineData(null, "required")] [InlineData("", "invalid_identifier")] [InlineData("é", "invalid_identifier")]
    [InlineData("a/b", "invalid_identifier")] [InlineData("a.b", "invalid_identifier")]
    public void Identifiers(string? id, string code)
    {
        var a = new RankingInput();
        foreach (var value in new[] { a with { Id = id }, a with { Dataset = id }, a with { Metadata = id }, a with { Strategy = id }, a with { Features = id }, a with { Model = id } }) Bad(value, code);
        if (id is not null) Bad(a with { Source = id }, code); else Assert.Empty(Syntax(a with { Source = null }));
        Assert.Empty(Syntax(a with { Id = new string('a', 64) })); Bad(a with { Id = new string('a', 65) }, "invalid_identifier");
    }
    [Theory] [InlineData(PolicyKind.FixedUniverse)] [InlineData(PolicyKind.SimpleRules)] [InlineData(PolicyKind.ModelRanking)]
    public void ModelVersionIsPresentOnlyForModelPolicy(PolicyKind policy)
    {
        var a = new RankingInput { Policy = policy, Model = policy == PolicyKind.ModelRanking ? "model" : null };
        Assert.Empty(Syntax(a));
        Bad(a with { Model = policy == PolicyKind.ModelRanking ? null : "model" }, policy == PolicyKind.ModelRanking ? "required" : "model_not_allowed");
    }
    [Fact] public void SchemaEnumsAndSlotsAreRequiredAndBounded()
    {
        Bad(new RankingInput { Schema = null }, "required"); Bad(new RankingInput { Schema = 2 }, "unsupported_schema_version");
        Bad(new RankingInput { Policy = null }, "required"); Bad(new RankingInput { Policy = (PolicyKind)99 }, "invalid_enum");
        Bad(new RankingInput { Evidence = null }, "required"); Bad(new RankingInput { Evidence = (EvidenceKind)99 }, "invalid_enum");
        Bad(new RankingInput { Slot = null }, "required"); Bad(new RankingInput { Slot = -1 }, "invalid_count");
        Assert.Empty(Syntax(new RankingInput { Slot = long.MaxValue }));
        foreach (var evidence in Enum.GetValues<EvidenceKind>()) Assert.Empty(Syntax(new RankingInput { Evidence = evidence }));
    }
    [Fact] public void EveryRequiredTimeRejectsNullAndNonUtcAndFutureTraining()
    {
        var a = new RankingInput(); var time = a.AsOf!.Value.ToOffset(TimeSpan.FromHours(1));
        foreach (var value in new[] { a with { TrainingStart = null }, a with { TrainingEnd = null }, a with { AsOf = null }, a with { From = null }, a with { Expiry = null } }) Bad(value, "required");
        foreach (var value in new[] { a with { TrainingStart = time }, a with { TrainingEnd = time }, a with { AsOf = time }, a with { From = time }, a with { Expiry = time } }) Bad(value, "invalid_utc");
        Bad(a with { TrainingStart = a.TrainingEnd }, "invalid_window"); Bad(a with { TrainingStart = a.TrainingEnd!.Value.AddTicks(1) }, "invalid_window");
        Bad(a with { TrainingEnd = a.AsOf!.Value.AddTicks(1) }, "future_training");
        Bad(a with { From = a.AsOf!.Value.AddTicks(-1) }, "invalid_validity");
        Bad(a with { Expiry = a.From }, "invalid_validity"); Bad(a with { Expiry = a.From!.Value.AddTicks(-1) }, "invalid_validity");
        Bad(a with { Expiry = a.From!.Value.AddHours(24).AddTicks(1) }, "invalid_validity");
        Assert.Empty(RankingArtifactValidator.Validate(a.Build(), Stage0Fixtures.Catalog())); // two-day training is supported
    }
    [Fact] public void TimeAwareValidityHasExactBoundariesAndChecksClock()
    {
        var a = new RankingInput(); var c = Stage0Fixtures.Catalog();
        Assert.Contains(RankingArtifactValidator.Validate(a.Build(), c, a.From!.Value.AddTicks(-1)), e => e.Code == "not_yet_valid");
        Assert.Empty(RankingArtifactValidator.Validate(a.Build(), c, a.From!.Value));
        Assert.Empty(RankingArtifactValidator.Validate(a.Build(), c, a.Expiry!.Value.AddTicks(-1)));
        Assert.Contains(RankingArtifactValidator.Validate(a.Build(), c, a.Expiry!.Value), e => e.Code == "expired");
        Assert.Contains(RankingArtifactValidator.Validate(a.Build(), c, a.From!.Value.ToOffset(TimeSpan.FromHours(1))), e => e.Field == "nowUtc" && e.Code == "invalid_utc");
    }
    [Fact] public void SharedRouteShapeAndDirection()
    {
        var a = Stage0Fixtures.A; var b = Stage0Fixtures.B; var p = Stage0Fixtures.P; var q = Stage0Fixtures.Q;
        ResearchRoute?[] invalid = [null, new(null, null), new([], []), new([a, a], [p]), new([a, b, a, b, a, a], [p, q, "R", "S", "T"]),
            new([a, b, b], [p, q]), new([a, b, a], [p]), new([a, b, a], [p, p]), new([a, null, a], [p, q]), new([a, b, a], [p, "bad/path"])];
        foreach (var route in invalid) Assert.NotEmpty(ResearchRouteSyntaxValidator.Validate(route));
        Assert.Empty(ResearchRouteSyntaxValidator.Validate(Stage0Fixtures.Route()));
        var four = new ResearchRoute([a, b, Stage0Fixtures.C, Stage0Fixtures.D, a], [p, Stage0Fixtures.R, Stage0Fixtures.S, Stage0Fixtures.T]);
        Assert.Empty(RankingArtifactValidator.Validate(Entry(new() { Route = four }).Build(), Stage0Fixtures.Catalog()));
        Assert.NotEqual(Canon(Entry(new())), Canon(Entry(new() { Route = Stage0Fixtures.Route(true) })));
    }
    [Fact] public void EntrySetLimitsContiguousRanksAndDuplicateRoutes()
    {
        Bad(new RankingInput { Entries = null }, "required"); Bad(new RankingInput { Entries = [] }, "invalid_entries"); Bad(new RankingInput { Entries = [null] }, "required");
        Bad(Entry(new() { Rank = null }), "required"); Bad(Entry(new() { Rank = 0 }), "invalid_rank"); Bad(Entry(new() { Rank = 2 }), "invalid_rank");
        var first = new RankingEntryInput();
        Bad(new RankingInput { Entries = [first.Build(), (first with { Rank = 2 }).Build()] }, "duplicate_route");
        Bad(new RankingInput { Entries = [first.Build(), (first with { Route = Stage0Fixtures.Route(true) }).Build()] }, "invalid_rank");
        var thousand = Enumerable.Range(1, 1000).Select(i => (RankingEntry?)new RankingEntryInput { Rank = i, Route = new(["A", "B", "A"], [$"P{i}", $"Q{i}"]) }.Build()).ToArray();
        Assert.Empty(Syntax(new RankingInput { Entries = thousand }));
        Bad(new RankingInput { Entries = [.. thousand, new RankingEntryInput { Rank = 1001 }.Build()] }, "invalid_entries");
    }
    [Fact] public void IntegerMetricBoundsAndNullability()
    {
        foreach (var entry in new[] { new RankingEntryInput { Min = null }, new RankingEntryInput { Max = null }, new RankingEntryInput { Priority = null }, new RankingEntryInput { Confidence = null }, new RankingEntryInput { Samples = null } }) Bad(Entry(entry), "required");
        foreach (var entry in new[] { new RankingEntryInput { Min = 0 }, new RankingEntryInput { Max = -1 }, new RankingEntryInput { Min = 2, Max = 1 } }) Bad(Entry(entry), "invalid_size");
        foreach (var priority in new[] { -1, 101 }) Bad(Entry(new() { Priority = priority }), "invalid_priority");
        foreach (var bp in new[] { -1, 10001 }) { Bad(Entry(new() { Confidence = bp }), "invalid_basis_points"); Bad(Entry(new() { Probability = bp }), "invalid_basis_points"); }
        Bad(Entry(new() { Samples = -1 }), "invalid_count"); Bad(Entry(new() { Route = null }), "required");
        foreach (var boundary in new[] { 0, 10000 }) Assert.Empty(Syntax(Entry(new() { Confidence = boundary, Probability = boundary })));
        Assert.Empty(Syntax(Entry(new() { Priority = 0, Probability = null, Value = null })));
        Assert.Empty(Syntax(Entry(new() { Priority = 100, Min = long.MaxValue, Max = long.MaxValue, Samples = long.MaxValue, Value = long.MinValue })));
    }
    [Fact] public void CatalogProvenanceCoverageAndRouteAdjacency()
    {
        var c = Stage0Fixtures.Catalog();
        void Check(RankingInput a, string code) => Assert.Contains(RankingArtifactValidator.Validate(a.Build(), c), i => i.Code == code);
        Check(new() { Dataset = "Synthetic-dataset" }, "unknown_dataset"); Check(new() { Metadata = "other" }, "metadata_version_mismatch");
        Check(new() { Strategy = "other" }, "unsupported_strategy"); Check(new() { TrainingStart = Stage0Fixtures.Start.AddTicks(-1) }, "invalid_window");
        Check(new() { TrainingEnd = Stage0Fixtures.Start.AddDays(11), AsOf = Stage0Fixtures.Start.AddDays(11), From = Stage0Fixtures.Start.AddDays(11), Expiry = Stage0Fixtures.Start.AddDays(12) }, "invalid_window");
        Check(Entry(new() { Route = new(["unknown", Stage0Fixtures.B, "unknown"], [Stage0Fixtures.P, Stage0Fixtures.Q]) }), "unknown_token");
        Check(Entry(new() { Route = new([Stage0Fixtures.A, Stage0Fixtures.B, Stage0Fixtures.A], ["unknown", Stage0Fixtures.Q]) }), "unknown_pool");
        Check(Entry(new() { Route = new([Stage0Fixtures.A, Stage0Fixtures.C, Stage0Fixtures.A], [Stage0Fixtures.P, Stage0Fixtures.Q]) }), "incompatible_pool");
        Check(Entry(new() { Route = new([Stage0Fixtures.A, Stage0Fixtures.A, Stage0Fixtures.A], [Stage0Fixtures.P, Stage0Fixtures.Q]) }), "incompatible_pool");
        Assert.Empty(RankingArtifactValidator.Validate(new RankingInput().Build(), Stage0Fixtures.Catalog(false)));
    }
    [Fact] public void QuoteBoundUsesFirstTokenAndDataset()
    {
        var a = Entry(new() { Max = 11 });
        Assert.Contains(RankingArtifactValidator.Validate(a.Build(), Stage0Fixtures.Catalog()), e => e.Code == "invalid_trade_size");
        Assert.Empty(RankingArtifactValidator.Validate(a.Build(), Stage0Fixtures.Catalog(quoteBound: 11)));
        Assert.Contains(RankingArtifactValidator.Validate(a.Build(), Stage0Fixtures.Catalog(quoteBound: 100, datasetBound: 10)), e => e.Code == "invalid_trade_size");
    }
    [Fact] public void SyntaxFirstReadOnlyIssuesAndInvalidCanonicalization()
    {
        var a = new RankingInput { Id = null }; var errors = Syntax(a);
        Assert.Equal(errors, RankingArtifactValidator.Validate(a.Build(), ResearchCatalog.Empty)); Assert.Equal(errors, Syntax(a));
        Assert.Throws<NotSupportedException>(() => ((IList<ValidationIssue>)errors).Clear());
        Assert.Throws<ArgumentNullException>(() => RankingArtifactValidator.Validate(a.Build(), null!));
        Assert.Equal("required", Assert.Single(RankingArtifactSyntaxValidator.Validate(null)).Code);
        Assert.Throws<ArgumentNullException>(() => RankingArtifactCanonicalizer.Canonicalize(null!));
        Assert.Throws<ArgumentException>(() => Canon(a));
    }
    [Fact] public void CanonicalOrderCultureIndependentScoresAndGoldenBytes()
    {
        var one = new RankingEntryInput(); var two = one with { Rank = 2, Route = Stage0Fixtures.Route(true), Probability = null, Value = long.MinValue };
        var a = new RankingInput { Entries = [two.Build(), one.Build()] };
        var before = a.Entries!.ToArray(); var canonical = RankingArtifactCanonicalizer.Canonicalize(a.Build()); Assert.Equal(1, canonical.Version);
        var culture = CultureInfo.CurrentCulture;
        try { CultureInfo.CurrentCulture = new("tr-TR"); Assert.Equal(canonical.Content, Canon(a with { Entries = [one.Build(), two.Build()] })); }
        finally { CultureInfo.CurrentCulture = culture; }
        Assert.Equal(before, a.Entries);
        using var json = JsonDocument.Parse(canonical.Content); var entries = json.RootElement.GetProperty("entries");
        Assert.Equal(8200, entries[0].GetProperty("researchConfidenceBps").GetInt32()); Assert.Equal(6100, entries[0].GetProperty("calibratedSuccessProbabilityBps").GetInt32());
        Assert.Equal(JsonValueKind.Null, entries[1].GetProperty("calibratedSuccessProbabilityBps").ValueKind); Assert.Equal(long.MinValue, entries[1].GetProperty("estimatedNetValueQuoteAtoms").GetInt64());
        Stage0Fixtures.Golden("2026-10-28-synthetic-ranking-artifact-v1.json", canonical.Content);
        foreach (var field in new[] { "artifactId", "datasetId", "datasetMetadataVersion", "strategyVersion", "featureSetVersion", "modelVersion", "sourceExperimentResultId" })
            Assert.StartsWith("synthetic-", json.RootElement.GetProperty(field).GetString());
    }
}
