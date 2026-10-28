using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

public class ExperimentResultContractTests
{
    static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-01-01T00:00:00Z", CultureInfo.InvariantCulture);
    static ExperimentRequest Request(string dataset = "synthetic-dataset", string strategy = "strategy-v1", long min = 1,
        string?[]? pools = null, DateTimeOffset? start = null, DateTimeOffset? end = null) =>
        new(dataset, strategy, start ?? Start, end ?? Start.AddHours(1), ["A", "B"], pools ?? ["P", "Q"], min, 10);
    static ResearchCatalog Catalog(bool available = true) => new([new("synthetic-dataset", "metadata-v1",
        "Synthetic contract fixture", available, "Contract tests only", Start, Start.AddHours(24), ["strategy-v1"],
        [new("A", 0, 10), new("B", 18, 10)], [new("P", ["A", "B"]), new("Q", ["B", "A"])], 10)]);
    static PolicyResult Policy(PolicyKind? kind = PolicyKind.FixedUniverse, long? evaluated = 10, long? detected = 4,
        long? selected = 2, IReadOnlyList<RejectedCandidateCount?>? rejected = null, long? missed = null,
        long? estimated = -5, long? simulated = null, long? realized = null) =>
        new(kind, evaluated, detected, selected, rejected ?? [new("z-reason", 1), new("A-reason", 2)],
            missed, estimated, simulated, realized);
    static PolicyResult?[] Policies(PolicyResult? first) => [first, Policy(PolicyKind.SimpleRules), Policy(PolicyKind.ModelRanking)];

    sealed record Submission
    {
        public string? Id { get; init; } = "synthetic-result";
        public string? Hash { get; init; } = RequestFingerprint.FromRequest(Request()).Sha256;
        public int? Version { get; init; } = 1;
        public string? Dataset { get; init; } = "synthetic-dataset";
        public string? Metadata { get; init; } = "metadata-v1";
        public string? Strategy { get; init; } = "strategy-v1";
        public string? Engine { get; init; } = "engine-v1";
        public string? Cost { get; init; } = "cost-v1";
        public DateTimeOffset? Created { get; init; } = DateTimeOffset.Parse("2026-10-27T00:00:00Z", CultureInfo.InvariantCulture);
        public DateTimeOffset? StartTime { get; init; } = Start;
        public DateTimeOffset? EndTime { get; init; } = Start.AddHours(1);
        public EvidenceKind? Evidence { get; init; } = EvidenceKind.Synthetic;
        public ExperimentResultStatus? Status { get; init; } = ExperimentResultStatus.Completed;
        public string? Error { get; init; }
        public IReadOnlyList<PolicyResult?>? Entries { get; init; } = Policies(Policy());
        public ExperimentResult Build() => new(Id, Hash, Version, Dataset, Metadata, Strategy, Engine, Cost,
            Created, StartTime, EndTime, Evidence, Status, Error, Entries);
    }
    static IReadOnlyList<ValidationIssue> Validate(Submission s) => ExperimentResultSyntaxValidator.Validate(s.Build());
    static string Canon(Submission s) => ExperimentResultCanonicalizer.Canonicalize(s.Build()).Content;
    static void Invalid(Submission s, string field, string code) => Assert.Contains(new ValidationIssue(field, code), Validate(s));

    [Fact] public void ValidSyntheticResultAndHistoricalAvailability()
    {
        var r = new Submission().Build();
        Assert.Empty(ExperimentResultSyntaxValidator.Validate(r));
        Assert.Empty(ExperimentResultValidator.Validate(r, Catalog(), Request()));
        Assert.Empty(ExperimentResultValidator.Validate(r, Catalog(false), Request()));
        Assert.Equal(new[] { "Synthetic", "Simulated", "Live" }, Enum.GetNames<EvidenceKind>());
        Assert.Equal(new[] { "Completed", "Failed", "Unavailable" }, Enum.GetNames<ExperimentResultStatus>());
        Assert.Equal(new[] { "FixedUniverse", "SimpleRules", "ModelRanking" }, Enum.GetNames<PolicyKind>());
    }

    [Fact] public void RawNullInvalidDuplicateAndOffsetInputsAreRetained()
    {
        var empty = new ExperimentResult(null, null, null, null, null, null, null, null, null, null, null, null, null, null, null);
        Assert.Null(empty.ResultId); Assert.Null(empty.RequestSha256); Assert.Null(empty.RequestCanonicalizationVersion);
        Assert.Null(empty.DatasetId); Assert.Null(empty.DatasetMetadataVersion); Assert.Null(empty.StrategyVersion);
        Assert.Null(empty.EngineVersion); Assert.Null(empty.CostModelVersion); Assert.Null(empty.CreatedAtUtc);
        Assert.Null(empty.WindowStartUtc); Assert.Null(empty.WindowEndUtc); Assert.Null(empty.EvidenceKind);
        Assert.Null(empty.Status); Assert.Null(empty.ErrorCode); Assert.Null(empty.Policies);
        var rejection = new RejectedCandidateCount("", -1);
        var rejections = new List<RejectedCandidateCount?> { rejection, null, rejection };
        var p = new PolicyResult((PolicyKind)99, -1, null, -2, rejections, -3, long.MinValue, -7, long.MaxValue);
        var entries = new List<PolicyResult?> { p, null, p };
        var time = Start.ToOffset(TimeSpan.FromHours(-5));
        var r = (new Submission { Id = " /bad ", Hash = "BAD", Version = -1, Created = time,
            StartTime = time, EndTime = time, Evidence = (EvidenceKind)99, Status = (ExperimentResultStatus)99,
            Error = "", Entries = entries }).Build();
        entries.Clear(); rejections.Clear();
        Assert.Equal(" /bad ", r.ResultId); Assert.Equal("BAD", r.RequestSha256); Assert.Equal(-1, r.RequestCanonicalizationVersion);
        Assert.Equal(time.Offset, r.CreatedAtUtc!.Value.Offset); Assert.Equal(time.Offset, r.WindowStartUtc!.Value.Offset);
        Assert.Equal((EvidenceKind)99, r.EvidenceKind); Assert.Equal((ExperimentResultStatus)99, r.Status); Assert.Equal("", r.ErrorCode);
        Assert.Equal(new PolicyResult?[] { p, null, p }, r.Policies);
        Assert.Equal(new RejectedCandidateCount?[] { rejection, null, rejection }, p.RejectedCandidates);
        Assert.Equal(-1, p.CandidatesEvaluated); Assert.Null(p.OpportunitiesDetected); Assert.Equal(-2, p.SelectedOrExecuted);
        Assert.Equal(-3, p.MissedCandidates); Assert.Equal(long.MinValue, p.EstimatedNetProfitQuoteAtoms);
        Assert.Equal(-7, p.SimulatedNetProfitQuoteAtoms); Assert.Equal(long.MaxValue, p.RealizedNetProfitQuoteAtoms);
        Assert.Throws<NotSupportedException>(() => ((IList<PolicyResult?>)r.Policies!).Clear());
        Assert.Throws<NotSupportedException>(() => ((IList<RejectedCandidateCount?>)p.RejectedCandidates!).Clear());
        Assert.Null(new PolicyResult(null, null, null, null, null, null, null, null, null).RejectedCandidates);
    }

    [Theory]
    [InlineData(null, "required")] [InlineData("", "invalid_identifier")]
    [InlineData(" ", "invalid_identifier")] [InlineData("é", "invalid_identifier")]
    [InlineData("a/b", "invalid_identifier")] [InlineData("a.b", "invalid_identifier")]
    public void IdentityValidation(string? value, string code)
    {
        var s = new Submission();
        Invalid(s with { Id = value }, "resultId", code); Invalid(s with { Dataset = value }, "datasetId", code);
        Invalid(s with { Metadata = value }, "datasetMetadataVersion", code); Invalid(s with { Strategy = value }, "strategyVersion", code);
        Invalid(s with { Engine = value }, "engineVersion", code); Invalid(s with { Cost = value }, "costModelVersion", code);
    }
    [Fact] public void IdentifierBoundaries()
    {
        Assert.Empty(Validate(new Submission { Id = new string('A', 64), Engine = "E-_09" }));
        Invalid(new Submission { Id = new string('A', 65) }, "resultId", "invalid_identifier");
    }
    [Theory]
    [InlineData(null, "required")] [InlineData("", "invalid_hash")]
    [InlineData("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "invalid_hash")]
    [InlineData("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "invalid_hash")]
    [InlineData("gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg", "invalid_hash")]
    public void HashValidation(string? hash, string code) => Invalid(new Submission { Hash = hash }, "requestSha256", code);
    [Theory] [InlineData(null, "required")] [InlineData(0, "unsupported_canonicalization_version")]
    [InlineData(2, "unsupported_canonicalization_version")]
    public void RequestVersionValidation(int? version, string code) => Invalid(new Submission { Version = version }, "requestCanonicalizationVersion", code);

    [Fact] public void UtcAndWindowBoundaries()
    {
        Invalid(new Submission { Created = null }, "createdAtUtc", "required");
        Invalid(new Submission { StartTime = null }, "windowStartUtc", "required");
        Invalid(new Submission { EndTime = null }, "windowEndUtc", "required");
        Invalid(new Submission { Created = Start.ToOffset(TimeSpan.FromHours(1)) }, "createdAtUtc", "invalid_utc");
        Invalid(new Submission { StartTime = Start.ToOffset(TimeSpan.FromHours(1)) }, "windowStartUtc", "invalid_utc");
        Invalid(new Submission { EndTime = Start.AddHours(1).ToOffset(TimeSpan.FromHours(-1)) }, "windowEndUtc", "invalid_utc");
        foreach (var end in new[] { Start, Start.AddTicks(-1), Start.AddHours(24).AddTicks(1) })
            Invalid(new Submission { EndTime = end }, "window", "invalid_window");
        Assert.Empty(Validate(new Submission { EndTime = Start.AddHours(24) }));
    }
    [Fact] public void NullAndUndefinedEnums()
    {
        Invalid(new Submission { Evidence = null }, "evidenceKind", "required");
        Invalid(new Submission { Evidence = (EvidenceKind)99 }, "evidenceKind", "invalid_enum");
        Invalid(new Submission { Status = null }, "status", "required");
        Invalid(new Submission { Status = (ExperimentResultStatus)99 }, "status", "invalid_enum");
        Invalid(new Submission { Entries = Policies(Policy(kind: null)) }, "policies[0].kind", "required");
        Invalid(new Submission { Entries = Policies(Policy(kind: (PolicyKind)99)) }, "policies[0].kind", "invalid_enum");
    }
    [Theory] [InlineData(ExperimentResultStatus.Failed)] [InlineData(ExperimentResultStatus.Unavailable)]
    public void NonCompletedHasErrorAndNoMetrics(ExperimentResultStatus status)
    {
        var s = new Submission { Status = status, Entries = null, Error = "engine_unavailable" };
        Assert.Empty(Validate(s)); Assert.Empty(Validate(s with { Entries = [] }));
        Invalid(s with { Error = null }, "errorCode", "required");
        Invalid(s with { Error = "" }, "errorCode", "invalid_identifier");
        Invalid(s with { Entries = Policies(Policy(estimated: null)) }, "policies", "metrics_not_allowed");
        using var json = JsonDocument.Parse(Canon(s));
        Assert.Equal(JsonValueKind.Null, json.RootElement.GetProperty("policies").ValueKind);
        Assert.Equal("engine_unavailable", json.RootElement.GetProperty("errorCode").GetString());
    }
    [Fact] public void CompletedHasEveryPolicyExactlyOnceAndNoError()
    {
        Invalid(new Submission { Error = "bad" }, "errorCode", "error_not_allowed");
        Invalid(new Submission { Entries = null }, "policies", "required");
        Invalid(new Submission { Entries = [] }, "policies", "missing_policy");
        Invalid(new Submission { Entries = [Policy(), Policy(PolicyKind.SimpleRules)] }, "policies", "missing_policy");
        Invalid(new Submission { Entries = [Policy(), Policy(), Policy(PolicyKind.ModelRanking)] }, "policies", "invalid_policy_set");
        Invalid(new Submission { Entries = Policies(null) }, "policies[0]", "required");
        Invalid(new Submission { Entries = [.. Policies(Policy()), Policy()] }, "policies", "invalid_policy_set");
        foreach (var kind in Enum.GetValues<PolicyKind>())
            Invalid(new Submission { Entries = Policies(Policy()).Where(p => p!.Kind != kind).ToArray() }, "policies", "missing_policy");
    }

    [Theory] [InlineData(EvidenceKind.Synthetic)] [InlineData(EvidenceKind.Simulated)] [InlineData(EvidenceKind.Live)]
    public void ProfitsStayIndependentIncludingNegativeAndLongBoundaries(EvidenceKind evidence)
    {
        var p = Policy(estimated: long.MinValue, simulated: evidence == EvidenceKind.Synthetic ? null : -7,
            realized: evidence == EvidenceKind.Live ? long.MaxValue : null);
        var s = new Submission { Evidence = evidence, Entries = Policies(p) };
        Assert.Empty(Validate(s));
        using var json = JsonDocument.Parse(Canon(s)); var policy = json.RootElement.GetProperty("policies")[0];
        Assert.Equal(long.MinValue, policy.GetProperty("estimatedNetProfitQuoteAtoms").GetInt64());
        if (evidence == EvidenceKind.Synthetic) Assert.Equal(JsonValueKind.Null, policy.GetProperty("simulatedNetProfitQuoteAtoms").ValueKind);
        else Assert.Equal(-7, policy.GetProperty("simulatedNetProfitQuoteAtoms").GetInt64());
        if (evidence == EvidenceKind.Live) Assert.Equal(long.MaxValue, policy.GetProperty("realizedNetProfitQuoteAtoms").GetInt64());
        else Assert.Equal(JsonValueKind.Null, policy.GetProperty("realizedNetProfitQuoteAtoms").ValueKind);
        Assert.Empty(Validate(s with { Entries = Policies(Policy(estimated: null)) }));
        // Single populated channels must never fill another channel.
        foreach (var only in new[] { Policy(estimated: 0), Policy(estimated: null, simulated: -1), Policy(estimated: null, realized: -2) })
        {
            var live = s with { Evidence = EvidenceKind.Live, Entries = Policies(only) };
            using var single = JsonDocument.Parse(Canon(live)); var item = single.RootElement.GetProperty("policies")[0];
            Assert.Equal(only.EstimatedNetProfitQuoteAtoms, item.GetProperty("estimatedNetProfitQuoteAtoms").Deserialize<long?>());
            Assert.Equal(only.SimulatedNetProfitQuoteAtoms, item.GetProperty("simulatedNetProfitQuoteAtoms").Deserialize<long?>());
            Assert.Equal(only.RealizedNetProfitQuoteAtoms, item.GetProperty("realizedNetProfitQuoteAtoms").Deserialize<long?>());
        }
    }
    [Theory] [InlineData(EvidenceKind.Synthetic)] [InlineData(EvidenceKind.Simulated)]
    public void RealizedRequiresLive(EvidenceKind evidence) => Invalid(new Submission { Evidence = evidence,
        Entries = Policies(Policy(realized: 0)) }, "policies[0].realizedNetProfitQuoteAtoms", "profit_not_allowed");
    [Fact] public void SyntheticForbidsSimulatedEvenZero() => Invalid(new Submission {
        Entries = Policies(Policy(simulated: 0)) }, "policies[0].simulatedNetProfitQuoteAtoms", "profit_not_allowed");

    [Theory] [InlineData(null, "required")] [InlineData(-1L, "invalid_count")]
    public void RequiredCountValidation(long? value, string code)
    {
        Invalid(new Submission { Entries = Policies(Policy(evaluated: value)) }, "policies[0].candidatesEvaluated", code);
        Invalid(new Submission { Entries = Policies(Policy(detected: value)) }, "policies[0].opportunitiesDetected", code);
        Invalid(new Submission { Entries = Policies(Policy(selected: value)) }, "policies[0].selectedOrExecuted", code);
        Invalid(new Submission { Entries = Policies(Policy(rejected: [new("reason", value)])) }, "policies[0].rejectedCandidates[0].count", code);
    }
    [Fact] public void OptionalMissedAndZeroCounts()
    {
        Invalid(new Submission { Entries = Policies(Policy(missed: -1)) }, "policies[0].missedCandidates", "invalid_count");
        foreach (long? missed in new long?[] { null, 0, long.MaxValue })
            Assert.Empty(Validate(new Submission { Entries = Policies(Policy(evaluated: 0, detected: 0, selected: 0, rejected: [], missed: missed)) }));
    }
    [Fact] public void RejectionReasonsAndMissingLists()
    {
        Invalid(new Submission { Entries = Policies(new(PolicyKind.FixedUniverse, 1, 1, 1, null, null, null, null, null)) }, "policies[0].rejectedCandidates", "required");
        Invalid(new Submission { Entries = Policies(Policy(rejected: [null])) }, "policies[0].rejectedCandidates[0]", "required");
        foreach (var reason in new string?[] { null, "", "bad/reason", new string('x', 65) })
            Invalid(new Submission { Entries = Policies(Policy(rejected: [new(reason, 1)])) }, "policies[0].rejectedCandidates[0].reasonCode", reason is null ? "required" : "invalid_identifier");
        Invalid(new Submission { Entries = Policies(Policy(rejected: [new("R", 1), new("R", 2)])) }, "policies[0].rejectedCandidates[1].reasonCode", "duplicate_reason");
        Assert.Empty(Validate(new Submission { Entries = Policies(Policy(rejected: [new("R", 1), new("r", 2)])) }));
    }
    [Fact] public void AccountingIsOverflowSafe()
    {
        var s = new Submission { Entries = Policies(Policy(evaluated: long.MaxValue, selected: 1, rejected: [new("R", long.MaxValue - 1)])) };
        Assert.Empty(Validate(s));
        foreach (var p in new[] { Policy(evaluated: 1, selected: 2, rejected: []),
            Policy(evaluated: 3, selected: 2, rejected: [new("R", 2)]),
            Policy(evaluated: long.MaxValue, selected: 1, rejected: [new("R", long.MaxValue)]),
            Policy(evaluated: long.MaxValue, selected: 0, rejected: [new("R", long.MaxValue), new("S", long.MaxValue)]) })
            Invalid(s with { Entries = Policies(p) }, "policies[0]", "candidate_count_exceeded");
    }

    [Fact] public void SyntaxPrecedesLookupAndIssuesAreImmutableDeterministic()
    {
        var r = new Submission { Id = "", Evidence = null, Hash = null }.Build();
        var syntax = ExperimentResultSyntaxValidator.Validate(r);
        Assert.Equal(3, syntax.Count); Assert.Equal(syntax, ExperimentResultSyntaxValidator.Validate(r));
        Assert.Equal(syntax, ExperimentResultValidator.Validate(r, ResearchCatalog.Empty));
        Assert.Equal(syntax, ExperimentResultValidator.Validate(r, ResearchCatalog.Empty, null));
        Assert.Throws<NotSupportedException>(() => ((IList<ValidationIssue>)syntax).Clear());
        Assert.Equal(new ValidationIssue("result", "required"), Assert.Single(ExperimentResultSyntaxValidator.Validate(null)));
        Assert.Throws<ArgumentNullException>(() => ExperimentResultValidator.Validate(r, null!));
    }
    [Fact] public void CatalogIdentityMetadataStrategyAndCoverage()
    {
        void Check(Submission s, string field, string code) => Assert.Contains(new ValidationIssue(field, code), ExperimentResultValidator.Validate(s.Build(), Catalog()));
        Assert.Equal("unknown_dataset", Assert.Single(ExperimentResultValidator.Validate(new Submission().Build(), ResearchCatalog.Empty)).Code);
        Check(new Submission { Dataset = "Synthetic-dataset" }, "datasetId", "unknown_dataset");
        Check(new Submission { Metadata = "Metadata-v1" }, "datasetMetadataVersion", "metadata_version_mismatch");
        Check(new Submission { Strategy = "Strategy-v1" }, "strategyVersion", "unsupported_strategy");
        Check(new Submission { StartTime = Start.AddTicks(-1) }, "window", "invalid_window");
        Check(new Submission { StartTime = Start.AddHours(23), EndTime = Start.AddHours(24).AddTicks(1) }, "window", "invalid_window");
        Assert.Empty(ExperimentResultValidator.Validate(new Submission { EndTime = Start.AddHours(24) }.Build(), Catalog()));
    }
    [Fact] public void FingerprintHashesActualCanonicalUtf8AndVerifiesScope()
    {
        var canonical = ExperimentRequestCanonicalizer.Canonicalize(Request());
        var fingerprint = RequestFingerprint.FromRequest(Request());
        Assert.Equal(canonical.Version, fingerprint.CanonicalizationVersion);
        Assert.Equal(Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(canonical.Content))), fingerprint.Sha256);
        Assert.Equal(fingerprint, RequestFingerprint.FromRequest(Request(pools: ["Q", "P"], start: Start.ToOffset(TimeSpan.FromHours(1)))));
        foreach (var request in new[] { Request(min: 2), Request(dataset: "different"), Request(strategy: "other"), Request(end: Start.AddHours(2)) })
            Assert.Contains(new ValidationIssue("request", "request_mismatch"), ExperimentResultValidator.Validate(new Submission().Build(), Catalog(), request));
        foreach (var changed in new[] { new Submission { Hash = new string('0', 64) }, new Submission { Dataset = "other" },
            new Submission { Strategy = "other" }, new Submission { EndTime = Start.AddHours(2) } })
            Assert.Contains(new ValidationIssue("request", "request_mismatch"), ExperimentResultValidator.Validate(changed.Build(), Catalog(), Request()));
        Assert.Contains(new ValidationIssue("request", "invalid_request"), ExperimentResultValidator.Validate(new Submission().Build(), Catalog(), null));
        Assert.Throws<ArgumentNullException>(() => RequestFingerprint.FromRequest(null!));
        Assert.Throws<ArgumentException>(() => RequestFingerprint.FromRequest(Request(min: 0)));
    }
    [Fact] public void CanonicalOrderingCultureExactBytesAndSharedScope()
    {
        var s = new Submission(); var before = s.Entries!.ToArray();
        var c = ExperimentResultCanonicalizer.Canonicalize(s.Build()); Assert.Equal(1, c.Version);
        var reordered = s with { Entries = [Policy(PolicyKind.ModelRanking, rejected: [new("A-reason", 2), new("z-reason", 1)]),
            Policy(PolicyKind.SimpleRules), Policy()] };
        var culture = CultureInfo.CurrentCulture;
        try { CultureInfo.CurrentCulture = new("tr-TR"); Assert.Equal(Encoding.UTF8.GetBytes(c.Content), Encoding.UTF8.GetBytes(Canon(reordered))); }
        finally { CultureInfo.CurrentCulture = culture; }
        Assert.Equal(before, s.Entries); Assert.Equal("z-reason", s.Entries![0]!.RejectedCandidates![0]!.ReasonCode);
        using var json = JsonDocument.Parse(c.Content); var root = json.RootElement;
        Assert.Equal(new[] { "canonicalizationVersion", "resultId", "requestSha256", "requestCanonicalizationVersion", "datasetId",
            "datasetMetadataVersion", "strategyVersion", "engineVersion", "costModelVersion", "createdAtUtc", "windowStartUtc",
            "windowEndUtc", "evidenceKind", "status", "errorCode", "policies" }, root.EnumerateObject().Select(p => p.Name));
        Assert.Equal("2026-10-27T00:00:00.0000000Z", root.GetProperty("createdAtUtc").GetString());
        Assert.Equal(Enum.GetNames<PolicyKind>(), root.GetProperty("policies").EnumerateArray().Select(p => p.GetProperty("kind").GetString()));
        foreach (var policy in root.GetProperty("policies").EnumerateArray())
        {
            Assert.Equal(new[] { "kind", "candidatesEvaluated", "opportunitiesDetected", "selectedOrExecuted", "rejectedCandidates",
                "missedCandidates", "estimatedNetProfitQuoteAtoms", "simulatedNetProfitQuoteAtoms", "realizedNetProfitQuoteAtoms" }, policy.EnumerateObject().Select(p => p.Name));
            Assert.False(policy.TryGetProperty("datasetId", out _)); Assert.False(policy.TryGetProperty("windowStartUtc", out _));
            Assert.False(policy.TryGetProperty("costModelVersion", out _));
            Assert.Equal(new[] { "A-reason", "z-reason" }, policy.GetProperty("rejectedCandidates").EnumerateArray().Select(r => r.GetProperty("reasonCode").GetString()));
        }
        Assert.NotEqual(c.Content, Canon(s with { Engine = "engine-v2" }));
        Assert.NotEqual(c.Content, Canon(s with { Entries = Policies(Policy(estimated: -6)) }));
    }
    [Fact] public void InvalidResultsCannotCanonicalize()
    {
        Assert.Throws<ArgumentNullException>(() => ExperimentResultCanonicalizer.Canonicalize(null!));
        foreach (var s in new[] { new Submission { Id = null }, new Submission { Entries = [] }, new Submission { Evidence = (EvidenceKind)99 },
            new Submission { Entries = Policies(Policy(realized: 1)) }, new Submission { Entries = Policies(Policy(evaluated: -1)) } })
            Assert.Throws<ArgumentException>(() => Canon(s));
    }
    [Fact] public void GoldenFixtureIsExplicitlySyntheticContractData()
    {
        var expected = File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", "2026-10-27-synthetic-experiment-result-v1.json"));
        Assert.Equal(expected, Encoding.UTF8.GetBytes(Canon(new Submission())));
        using var json = JsonDocument.Parse(expected); var root = json.RootElement;
        Assert.Equal("Synthetic", root.GetProperty("evidenceKind").GetString());
        Assert.StartsWith("synthetic-", root.GetProperty("resultId").GetString());
        Assert.StartsWith("synthetic-", root.GetProperty("datasetId").GetString());
        Assert.Equal(RequestFingerprint.FromRequest(Request()).Sha256, root.GetProperty("requestSha256").GetString());
    }
}
