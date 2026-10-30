using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. The route fixture is generated from the .NET v2 enumerator.
public class MultiRouteReplayContractTests
{
    static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-10-30T16:24:16.0436210Z");
    static readonly DateTimeOffset End = DateTimeOffset.Parse("2026-10-30T16:32:14.5720580Z");
    static ExperimentRequestV2 Request() => new("orca-wp-snapshot-20261030", CaptureReplayCatalog.StrategyVersion,
        Start, End, CaptureReplayCatalog.Usdc,
        [CaptureReplayCatalog.Sol, CaptureReplayCatalog.Usdc, CaptureReplayCatalog.Usdt],
        CaptureReplayCatalog.Pools('a').Concat(CaptureReplayCatalog.Pools('b')).Concat(CaptureReplayCatalog.Pools('c')).ToArray(),
        ["two_leg", "triangle"], ["83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d"], 10_000_000, 1_000_000_000);

    static string Fixture(string kind) => File.ReadAllText(Path.Combine(AppContext.BaseDirectory,
        "Fixtures", $"2026-10-30-multi-route-v2-{kind}.json"));
    static ExperimentRequestV2 ReadRequest(JsonElement root) => new(
        root.GetProperty("datasetId").GetString(), root.GetProperty("strategyVersion").GetString(),
        root.GetProperty("windowStartUtc").GetDateTimeOffset(), root.GetProperty("windowEndUtc").GetDateTimeOffset(),
        root.GetProperty("quoteTokenId").GetString(),
        root.GetProperty("tokenIds").EnumerateArray().Select(v => v.GetString()).ToArray(),
        root.GetProperty("poolIds").EnumerateArray().Select(v => v.GetString()).ToArray(),
        root.GetProperty("routeShapes").EnumerateArray().Select(v => v.GetString()).ToArray(),
        root.GetProperty("priceInputPoolIds").EnumerateArray().Select(v => v.GetString()).ToArray(),
        root.GetProperty("minTradeSizeQuoteAtoms").GetInt64(), root.GetProperty("maxTradeSizeQuoteAtoms").GetInt64());
    static ExperimentResult ReadResult(JsonElement root)
    {
        var policies = root.GetProperty("policies").EnumerateArray().Select(p => new PolicyResult(
            Enum.Parse<PolicyKind>(p.GetProperty("kind").GetString()!), p.GetProperty("candidatesEvaluated").GetInt64(),
            p.GetProperty("opportunitiesDetected").GetInt64(), p.GetProperty("selectedOrExecuted").GetInt64(),
            p.GetProperty("rejectedCandidates").EnumerateArray().Select(r => (RejectedCandidateCount?)new RejectedCandidateCount(
                r.GetProperty("reasonCode").GetString(), r.GetProperty("count").GetInt64())).ToArray(),
            null, p.GetProperty("estimatedNetProfitQuoteAtoms").GetInt64(),
            p.GetProperty("simulatedNetProfitQuoteAtoms").GetInt64(), null)).Cast<PolicyResult?>().ToArray();
        return new ExperimentResult(root.GetProperty("resultId").GetString(), root.GetProperty("requestSha256").GetString(),
            root.GetProperty("requestCanonicalizationVersion").GetInt32(), root.GetProperty("datasetId").GetString(),
            root.GetProperty("datasetMetadataVersion").GetString(), root.GetProperty("strategyVersion").GetString(),
            root.GetProperty("engineVersion").GetString(), root.GetProperty("costModelVersion").GetString(),
            root.GetProperty("createdAtUtc").GetDateTimeOffset(), root.GetProperty("windowStartUtc").GetDateTimeOffset(),
            root.GetProperty("windowEndUtc").GetDateTimeOffset(), Enum.Parse<EvidenceKind>(root.GetProperty("evidenceKind").GetString()!),
            Enum.Parse<ExperimentResultStatus>(root.GetProperty("status").GetString()!), null, policies);
    }

    [Fact] public void CapturedV2RequestAndResultMatchCanonicalContracts()
    {
        using var requestJson = JsonDocument.Parse(Fixture("request"));
        using var resultJson = JsonDocument.Parse(Fixture("result"));
        var request = ReadRequest(requestJson.RootElement);
        var result = ReadResult(resultJson.RootElement);
        var catalog = CaptureReplayCatalog.CreateSnapshot();
        Assert.Equal(2, requestJson.RootElement.GetProperty("canonicalizationVersion").GetInt32());
        Assert.Equal(1, resultJson.RootElement.GetProperty("canonicalizationVersion").GetInt32());
        Assert.Empty(ExperimentRequestV2Validator.Validate(request, catalog));
        Assert.Equal(Fixture("request"), ExperimentRequestV2Canonicalizer.Canonicalize(request).Content);
        var requestHash = Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(Fixture("request"))));
        Assert.Equal(requestHash, RequestFingerprint.FromRequest(request).Sha256);
        Assert.Equal(requestHash, result.RequestSha256);
        Assert.Equal(152, ExperimentRequestV2RouteEnumerator.Enumerate(request, catalog).Count);
        Assert.Empty(ExperimentResultV2SyntaxValidator.Validate(result));
        Assert.Empty(ExperimentResultValidator.Validate(result, catalog, request));
        Assert.Equal(Fixture("result"), ExperimentResultV2Canonicalizer.Canonicalize(result).Content);
        Assert.Equal(EvidenceKind.Simulated, result.EvidenceKind);
        Assert.Equal("research-replay-v2-0_1_0", result.EngineVersion);
    }

    [Fact] public void FullOrderedR2FixtureMatchesDotNetEnumerator()
    {
        var routes = ExperimentRequestV2RouteEnumerator.Enumerate(Request(), CaptureReplayCatalog.CreateSnapshot());
        Assert.Equal(152, routes.Count);
        Assert.Equal(120, routes.Count(r => r.Shape == "triangle"));
        Assert.Equal(32, routes.Count(r => r.Shape == "two_leg"));
        var actual = JsonSerializer.SerializeToUtf8Bytes(routes.Select(r => new {shape=r.Shape,pools=r.PoolIds,tokens=r.TokenIds}));
        var fixture = File.ReadAllBytes(Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,
            "../../../../../../crates/research-replay/tests/fixtures/2026-10-30-r2-ordered-routes.json")));
        Assert.Equal(fixture, actual);
    }
}
