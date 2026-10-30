using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

public class CaptureReplayResultTests
{
    static string Fixture(char pair, string kind) => File.ReadAllText(Path.Combine(AppContext.BaseDirectory,
        "Fixtures", $"2026-10-30-replay-{pair}-{kind}.json"));
    static ExperimentRequest ReadRequest(JsonElement root) => new(
        root.GetProperty("datasetId").GetString(), root.GetProperty("strategyVersion").GetString(),
        root.GetProperty("windowStartUtc").GetDateTimeOffset(), root.GetProperty("windowEndUtc").GetDateTimeOffset(),
        root.GetProperty("tokenPair").EnumerateArray().Select(v => v.GetString()).ToArray(),
        root.GetProperty("poolIds").EnumerateArray().Select(v => v.GetString()).ToArray(),
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

    [Theory]
    [InlineData('a', CaptureReplayCatalog.Sol, CaptureReplayCatalog.Usdc, 6)]
    [InlineData('b', CaptureReplayCatalog.Sol, CaptureReplayCatalog.Usdt, 5)]
    [InlineData('c', CaptureReplayCatalog.Usdc, CaptureReplayCatalog.Usdt, 2)]
    public void RealCapturePairValidatesUnchangedV1(char pair, string baseToken, string quoteToken, int poolCount)
    {
        using var requestJson = JsonDocument.Parse(Fixture(pair, "request"));
        using var resultJson = JsonDocument.Parse(Fixture(pair, "result"));
        var request = ReadRequest(requestJson.RootElement);
        var result = ReadResult(resultJson.RootElement);
        var catalog = CaptureReplayCatalog.Create();
        Assert.Equal(new[] { baseToken, quoteToken }, request.TokenPair);
        Assert.Equal(CaptureReplayCatalog.Pools(pair).Order(StringComparer.Ordinal), request.PoolIds);
        Assert.Equal(poolCount, request.PoolIds!.Count);
        Assert.Empty(ExperimentRequestSyntaxValidator.Validate(request));
        Assert.Empty(ExperimentRequestValidator.Validate(request, catalog));
        Assert.Equal(Fixture(pair, "request"), ExperimentRequestCanonicalizer.Canonicalize(request).Content);
        Assert.Empty(ExperimentResultSyntaxValidator.Validate(result));
        Assert.Empty(ExperimentResultValidator.Validate(result, catalog, request));
        Assert.Equal(Fixture(pair, "result"), ExperimentResultCanonicalizer.Canonicalize(result).Content);
        Assert.Equal(EvidenceKind.Simulated, result.EvidenceKind);
        Assert.All(result.Policies!, p => Assert.Null(p!.RealizedNetProfitQuoteAtoms));
    }
}
