using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. Version-two request linkage does not widen the v1 result syntax path.
public class ExperimentResultV2Tests
{
    static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-10-30T16:24:16.0436210Z");
    static readonly DateTimeOffset End = DateTimeOffset.Parse("2026-10-30T16:32:14.5720580Z");
    static ExperimentResult Result(int? version) => new("synthetic-v2-result", new string('a', 64), version,
        "orca-wp-snapshot-20261030", "capture-v2", CaptureReplayCatalog.StrategyVersion,
        "research-replay-v2-0_1_0", "cost-v2-usdc-83v8iPyZihDE-10000", End, Start, End,
        EvidenceKind.Simulated, ExperimentResultStatus.Completed, null,
        Enum.GetValues<PolicyKind>().Select(kind => (PolicyResult?)new PolicyResult(kind, 0, 0, 0, [], null, 0, 0, null)).ToArray());

    [Fact] public void VersionTwoSyntaxAndCanonicalBytes()
    {
        var result = Result(2);
        Assert.Empty(ExperimentResultV2SyntaxValidator.Validate(result));
        Assert.Contains(new ValidationIssue("requestCanonicalizationVersion", "unsupported_canonicalization_version"),
            ExperimentResultSyntaxValidator.Validate(result));
        var canonical = ExperimentResultV2Canonicalizer.Canonicalize(result);
        Assert.Equal(1, canonical.Version);
        using var json = JsonDocument.Parse(canonical.Content);
        Assert.Equal(1, json.RootElement.GetProperty("canonicalizationVersion").GetInt32());
        Assert.Equal(2, json.RootElement.GetProperty("requestCanonicalizationVersion").GetInt32());
        Assert.Equal("2026-10-30T16:24:16.0436210Z", json.RootElement.GetProperty("windowStartUtc").GetString());
        Assert.Equal(Encoding.UTF8.GetBytes(canonical.Content), Encoding.UTF8.GetBytes(ExperimentResultV2Canonicalizer.Canonicalize(result).Content));
        Assert.Throws<ArgumentException>(() => ExperimentResultCanonicalizer.Canonicalize(result));
    }

    [Fact] public void VersionOneAndInvalidResultsAreRejectedByV2()
    {
        Assert.Equal(new ValidationIssue("result", "required"), Assert.Single(ExperimentResultV2SyntaxValidator.Validate(null)));
        foreach (var version in new int?[] { null, 1, 3 })
        {
            var result = Result(version);
            Assert.Contains(ExperimentResultV2SyntaxValidator.Validate(result), i => i.Field == "requestCanonicalizationVersion");
            Assert.Throws<ArgumentException>(() => ExperimentResultV2Canonicalizer.Canonicalize(result));
        }
        Assert.Throws<ArgumentNullException>(() => ExperimentResultV2Canonicalizer.Canonicalize(null!));
    }
}
