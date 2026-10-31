using System.Security.Cryptography;
using System.Text;
using MEVana.Research.Api;
using MEVana.Research.Core;
using MEVana.Research.Persistence;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. Each tamper case works on a temporary copy of the committed seed.
public sealed class ResearchArtifactStoreTests : IDisposable
{
    static readonly string Seed = Path.Combine(AppContext.BaseDirectory, "research-artifacts");
    const string V1Request = "2026-10-30-snapshot-replay-a-request.json";
    const string V1Result = "2026-10-30-snapshot-replay-a-result.json";
    const string V2Request = "2026-10-30-multi-route-v2-request.json";
    const string V2Result = "2026-10-30-multi-route-v2-result.json";
    const string V2Summary = "2026-10-30-multi-route-v2-summary.json";
    readonly string _copy = Directory.CreateTempSubdirectory("research-artifacts-").FullName;

    public ResearchArtifactStoreTests()
    {
        foreach (var file in Directory.GetFiles(Seed)) File.Copy(file, Path.Combine(_copy, Path.GetFileName(file)));
    }
    public void Dispose() => Directory.Delete(_copy, true);

    string At(string file) => Path.Combine(_copy, file);
    void Replace(string file, string from, string to)
    {
        var text = File.ReadAllText(At(file));
        Assert.Contains(from, text);
        File.WriteAllText(At(file), text.Replace(from, to));
    }
    void RelinkRequestHash(string requestFile, string resultFile)
    {
        var current = Convert.ToHexStringLower(SHA256.HashData(File.ReadAllBytes(Path.Combine(Seed, requestFile))));
        var updated = Convert.ToHexStringLower(SHA256.HashData(File.ReadAllBytes(At(requestFile))));
        Replace(resultFile, current, updated);
    }
    void Fails(string code) =>
        Assert.Equal(code, Assert.Throws<ArtifactIntegrityException>(() => ResearchArtifactStore.Load(_copy)).Code);

    [Fact] public void SeedLoadsExactlyTheFourExperiments()
    {
        var store = ResearchArtifactStore.Load(Seed);
        Assert.Equal(new[]
        {
            "replay-v2-463353643-a-confirmed-l1-tip10000", "replay-v2-463353643-b-confirmed-l1-tip10000",
            "replay-v2-463353643-c-confirmed-l1-tip10000", "replay-v2-463353643-usdc-confirmed-l1-tip10000"
        }, store.Experiments.Select(e => e.Result.ResultId));
        Assert.All(store.Experiments, e => Assert.Equal(EvidenceKind.Simulated, e.Result.EvidenceKind));
    }

    [Theory]
    [InlineData("2026-10-30-snapshot-replay-a-request.json")] [InlineData("2026-10-30-snapshot-replay-a-result.json")]
    [InlineData("2026-10-30-snapshot-replay-b-request.json")] [InlineData("2026-10-30-snapshot-replay-b-result.json")]
    [InlineData("2026-10-30-snapshot-replay-c-request.json")] [InlineData("2026-10-30-snapshot-replay-c-result.json")]
    [InlineData("2026-10-30-multi-route-v2-request.json")] [InlineData("2026-10-30-multi-route-v2-result.json")]
    public void SeedCopiesMatchFixturesByteForByte(string file) => Assert.Equal(
        File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", file)), File.ReadAllBytes(Path.Combine(Seed, file)));

    [Fact] public void SummaryIsTheHandoff14Artifact() =>
        Assert.StartsWith("8edff656b82130ee", Convert.ToHexStringLower(SHA256.HashData(File.ReadAllBytes(Path.Combine(Seed, V2Summary)))));

    [Fact] public void CatalogMatchesTheSnapshotCatalog()
    {
        var expected = Assert.Single(CaptureReplayCatalog.CreateSnapshot().Datasets);
        var actual = Assert.Single(ResearchArtifactStore.Load(Seed).Catalog.Datasets);
        Assert.Equal((expected.Id, expected.MetadataVersion, expected.Provenance, expected.VerifiedAvailable, expected.Suitability,
                expected.CoverageStartUtc, expected.CoverageEndUtc, expected.MaxResearchTradeSizeQuoteAtoms),
            (actual.Id, actual.MetadataVersion, actual.Provenance, actual.VerifiedAvailable, actual.Suitability,
                actual.CoverageStartUtc, actual.CoverageEndUtc, actual.MaxResearchTradeSizeQuoteAtoms));
        Assert.Equal(expected.SupportedStrategyVersions, actual.SupportedStrategyVersions);
        Assert.Equal(expected.Tokens.Select(t => (t.Id, t.Decimals, t.MaxResearchTradeSizeQuoteAtoms)),
            actual.Tokens.Select(t => (t.Id, t.Decimals, t.MaxResearchTradeSizeQuoteAtoms)));
        Assert.Equal(expected.Pools.Select(p => (p.Id, string.Join(",", p.TokenIds))), actual.Pools.Select(p => (p.Id, string.Join(",", p.TokenIds))));
    }

    [Fact] public void TruncatedDocumentFails()
    {
        File.WriteAllBytes(At(V1Result), File.ReadAllBytes(At(V1Result))[..40]);
        Fails("invalid_json");
    }

    [Fact] public void MissingFileFails()
    {
        File.Delete(At(V2Summary));
        Fails("missing_file");
    }

    [Theory] [InlineData("../catalog.json")] [InlineData("/etc/passwd")] [InlineData("a\\\\b.json")]
    public void ManifestFilePathsCannotLeaveTheDirectory(string name)
    {
        Replace("manifest.json", V1Result, name);
        Fails("invalid_file_name");
    }

    [Fact] public void ChangedRequestByteFailsTheHashCheck()
    {
        Replace(V1Request, "\"minTradeSizeQuoteAtoms\":10000000", "\"minTradeSizeQuoteAtoms\":10000001");
        Fails("request_hash_mismatch");
    }

    [Fact] public void WrongHashFails()
    {
        var hash = Convert.ToHexStringLower(SHA256.HashData(File.ReadAllBytes(At(V2Request))));
        Replace(V2Result, hash, new string('0', 64));
        Fails("request_hash_mismatch");
    }

    [Fact] public void NoncanonicalRequestFailsEvenWithAMatchingHash()
    {
        Replace(V1Request, "{\"canonicalizationVersion\"", "{ \"canonicalizationVersion\"");
        RelinkRequestHash(V1Request, V1Result);
        Fails("noncanonical_bytes");
    }

    [Fact] public void NoncanonicalResultFails()
    {
        Replace(V1Result, "{\"canonicalizationVersion\"", "{ \"canonicalizationVersion\"");
        Fails("noncanonical_bytes");
    }

    [Fact] public void RequestOutsideTheCatalogFailsContractValidation()
    {
        Replace("catalog.json", "ranking-v0-heuristic", "other-strategy");
        Fails("contract_validation_failed");
    }

    [Fact] public void LiveEvidenceFails()
    {
        Replace(V1Result, "\"evidenceKind\":\"Simulated\"", "\"evidenceKind\":\"Live\"");
        Fails("unsupported_evidence");
    }

    [Fact] public void RealizedProfitFails()
    {
        var text = File.ReadAllText(At(V1Result));
        var index = text.IndexOf("\"realizedNetProfitQuoteAtoms\":null", StringComparison.Ordinal);
        Assert.True(index > 0);
        File.WriteAllText(At(V1Result), text[..index] + "\"realizedNetProfitQuoteAtoms\":1" + text[(index + "\"realizedNetProfitQuoteAtoms\":null".Length)..]);
        Fails("unsupported_evidence");
    }

    [Fact] public void UnknownPropertyFails()
    {
        Replace(V1Result, "\"errorCode\":null", "\"errorCode\":null,\"extra\":null");
        Fails("unknown_property");
    }

    [Fact] public void DuplicatePropertyFails()
    {
        Replace(V1Result, "\"errorCode\":null", "\"errorCode\":null,\"errorCode\":null");
        Fails("duplicate_property");
    }

    [Theory] [InlineData("9007199254740992")] [InlineData("-9007199254740992")] [InlineData("1.5")]
    public void UnsafeNumberFails(string value)
    {
        Replace(V2Summary, "\"slotBoundaries\":2235", $"\"slotBoundaries\":{value}");
        Fails("unsafe_integer");
    }

    [Fact] public void SummaryThatDoesNotSumFails()
    {
        Replace(V2Summary, "\"candidatesEvaluated\":268200", "\"candidatesEvaluated\":268199");
        Fails("summary_mismatch");
    }

    [Fact] public void SummaryOnAV1ExperimentFails()
    {
        Replace("manifest.json", "\"summaryFile\": null", $"\"summaryFile\": \"{V2Summary}\"");
        Fails("summary_mismatch");
    }

    [Fact] public void DuplicateExperimentFails()
    {
        Replace("manifest.json", "\"experiments\": [", """
            "experiments": [ { "resultId": "replay-v2-463353643-a-confirmed-l1-tip10000", "requestContractVersion": 1,
            "requestFile": "2026-10-30-snapshot-replay-a-request.json", "resultFile": "2026-10-30-snapshot-replay-a-result.json",
            "summaryFile": null, "sourceCloseout": "x", "limitations": [] },
            """);
        Fails("duplicate_experiment");
    }

    [Fact] public void EachRequestParserRefusesTheOtherContractVersion()
    {
        var v1 = File.ReadAllBytes(At(V1Request));
        var v2 = File.ReadAllBytes(At(V2Request));
        Assert.Equal("contract_version_mismatch", Assert.Throws<ArtifactIntegrityException>(() => ResearchArtifactParser.ParseRequestV1(v2)).Code);
        Assert.Equal("contract_version_mismatch", Assert.Throws<ArtifactIntegrityException>(() => ResearchArtifactParser.ParseRequestV2(v1)).Code);
    }

    [Fact] public void ManifestContractVersionCannotCrossTheParsers()
    {
        Replace("manifest.json", "\"requestContractVersion\": 2", "\"requestContractVersion\": 1");
        Fails("contract_version_mismatch");
    }

    [Fact] public async Task FailedArtifactsPreventTheServerFromStarting()
    {
        Replace(V1Result, "\"evidenceKind\":\"Simulated\"", "\"evidenceKind\":\"Live\"");
        await Assert.ThrowsAsync<ArtifactIntegrityException>(() => ResearchApiApp.StartAsync(new(_copy, "http://127.0.0.1:0")));
    }
}
