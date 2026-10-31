using System.Text.Json.Nodes;
using MEVana.Research.Persistence;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. Each test tampers with an isolated copy of the approved seed.
public sealed class ResearchSeedPinTests : IDisposable
{
    const string Summary = "2026-10-30-multi-route-v2-summary.json";
    const string PairBResult = "2026-10-30-snapshot-replay-b-result.json";
    readonly string _copy = Directory.CreateTempSubdirectory("research-seed-pin-").FullName;

    public ResearchSeedPinTests()
    {
        var seed = Path.Combine(AppContext.BaseDirectory, "research-artifacts");
        foreach (var file in Directory.GetFiles(seed))
            File.Copy(file, Path.Combine(_copy, Path.GetFileName(file)));
    }

    public void Dispose() => Directory.Delete(_copy, true);

    string At(string file) => Path.Combine(_copy, file);

    void Fails(string file)
    {
        var failure = Assert.Throws<ArtifactIntegrityException>(() => ResearchArtifactStore.Load(_copy));
        Assert.Equal("seed_hash_mismatch", failure.Code);
        Assert.Equal(file, failure.Artifact);
    }

    void EditJson(string file, Action<JsonNode> edit)
    {
        var root = JsonNode.Parse(File.ReadAllText(At(file)))!;
        edit(root);
        File.WriteAllText(At(file), root.ToJsonString());
    }

    void Replace(string file, string before, string after)
    {
        var content = File.ReadAllText(At(file));
        Assert.Contains(before, content);
        File.WriteAllText(At(file), content.Replace(before, after, StringComparison.Ordinal));
    }

    [Fact]
    public void NullPoolDirectionsFails()
    {
        EditJson(Summary, root => root["poolDirections"] = null);
        Fails(Summary);
    }

    [Fact]
    public void ChangedProbeInputAtomsFails()
    {
        EditJson(Summary, root => root["probeInputAtoms"] = new JsonArray(1, 2, 3));
        Fails(Summary);
    }

    [Fact]
    public void MovedShapeCountsFailEvenWhenPolicyTotalsArePreserved()
    {
        EditJson(Summary, root =>
        {
            var shapes = root["shapes"]!;
            foreach (var policy in shapes["triangle"]!["policies"]!.AsArray())
                policy!["candidatesEvaluated"] = 268199;
            foreach (var policy in shapes["two_leg"]!["policies"]!.AsArray())
                policy!["candidatesEvaluated"] = 71521;
        });
        Fails(Summary);
    }

    [Fact]
    public void RemovedManifestEntryFails()
    {
        EditJson("manifest.json", root => root["experiments"]!.AsArray().RemoveAt(0));
        Fails("manifest.json");
    }

    [Fact]
    public void ChangedSimulatedProfitFails()
    {
        Replace(PairBResult, "\"simulatedNetProfitQuoteAtoms\":0", "\"simulatedNetProfitQuoteAtoms\":1");
        Fails(PairBResult);
    }

    [Fact]
    public void ChangedTokenDecimalsFail()
    {
        Replace("catalog.json", "\"decimals\": 9", "\"decimals\": 8");
        Fails("catalog.json");
    }
}
