using System.Collections.ObjectModel;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MEVana.Research.Core;

namespace MEVana.Research.Persistence;

/// <summary>One validated experiment. All fields are fixed once loaded.</summary>
public sealed record StoredExperiment(
    ExperimentResult Result, int RequestContractVersion, string RequestJson, string ResultJson,
    string QuoteTokenId, int QuoteTokenDecimals, IReadOnlyList<string> RouteShapes,
    IReadOnlyList<string> TokenIds, int PoolCount, IReadOnlyList<ExperimentRouteV2>? Routes,
    string? ShapesJson, string? PoolDirectionsJson, string? ProbeInputAtomsJson,
    string SourceCloseout, IReadOnlyList<string> Limitations);

/// <summary>
/// Loads seed artifacts once at startup. Any failed check throws, so nothing is served.
/// Request handlers read only these in-memory values.
/// </summary>
public sealed class ResearchArtifactStore
{
    static readonly IReadOnlyDictionary<string, string> SeedHashes = new ReadOnlyDictionary<string, string>(
        new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["2026-10-30-multi-route-v2-request.json"] = "63788e9af74985533851fbeb6f2d24481d3f6c87e3acadcecb7387ef2b713953",
            ["2026-10-30-multi-route-v2-result.json"] = "a4223ade793105e3c060a6b1d711d02993f94161312e8ef44491f5d94bf4dd09",
            ["2026-10-30-multi-route-v2-summary.json"] = "8edff656b82130ee7006d90687b3a303a85052b0fd20f52482b30b3e32c5f599",
            ["2026-10-30-snapshot-replay-a-request.json"] = "474dec245a21450366f9339b20d58ffc3962c04f5c99e049e7d33bd2c9a8c373",
            ["2026-10-30-snapshot-replay-a-result.json"] = "00d36a8a44df3ed53b1c475b045fc0cf13470e888cefa472fd73265581191a5f",
            ["2026-10-30-snapshot-replay-b-request.json"] = "7c20450f5289fa9c1f19804f49e161b4297a900f8ac3ba47b1c5f129ce91117d",
            ["2026-10-30-snapshot-replay-b-result.json"] = "bd1bfa458a60934e8b5aa4fdf0eed5ddc89c414ef4d1f176177128582cb0fd47",
            ["2026-10-30-snapshot-replay-c-request.json"] = "868c9996c6d491d53c0ddc5adb957b19c796ded330df23d1d000723b977ac414",
            ["2026-10-30-snapshot-replay-c-result.json"] = "b211afe876d4769d15b242d6a6ef52e96b86129ea2a6512c2cc3c5a925c07d85",
            ["catalog.json"] = "b3c46a902b33eb4f6ff86dd7ddf38c091cb2504a692d0dff45b2158aebc7df02",
            ["manifest.json"] = "920844455dbfd6df1064fce50a26005e4158ba482642a8b8425535e904cbf3bc"
        });

    ResearchArtifactStore(ResearchCatalog catalog, IReadOnlyList<StoredExperiment> experiments)
    {
        Catalog = catalog;
        Experiments = experiments;
    }

    public ResearchCatalog Catalog { get; }
    public IReadOnlyList<StoredExperiment> Experiments { get; }

    public static ResearchArtifactStore Load(string directory)
    {
        var catalog = ResearchArtifactParser.ParseCatalog(Read(directory, "catalog.json"), "catalog.json");
        var manifest = new ArtifactJson("manifest.json");
        using var document = manifest.Parse(Read(directory, "manifest.json"));
        manifest.Exact(document.RootElement, "experiments");
        var experiments = manifest.Items(document.RootElement, "experiments")
            .Select(entry => LoadExperiment(directory, catalog, manifest, entry))
            .OrderBy(e => e.Result.ResultId, StringComparer.Ordinal).ToArray();
        if (experiments.Select(e => e.Result.ResultId).Distinct(StringComparer.Ordinal).Count() != experiments.Length)
            throw manifest.Fail("duplicate_experiment");
        foreach (var (file, expectedHash) in SeedHashes)
            if (!string.Equals(Convert.ToHexStringLower(SHA256.HashData(Read(directory, file))), expectedHash, StringComparison.Ordinal))
                throw new ArtifactIntegrityException("seed_hash_mismatch", file);
        return new(catalog, Array.AsReadOnly(experiments));
    }

    static StoredExperiment LoadExperiment(string directory, ResearchCatalog catalog, ArtifactJson manifest, JsonElement entry)
    {
        manifest.Exact(entry, "resultId", "requestContractVersion", "requestFile", "resultFile",
            "summaryFile", "sourceCloseout", "limitations");
        var contract = (int)manifest.Number(entry, "requestContractVersion");
        var requestFile = manifest.Text(entry, "requestFile");
        var resultFile = manifest.Text(entry, "resultFile");
        var summaryFile = manifest.OptionalText(entry, "summaryFile");
        var requestBytes = Read(directory, requestFile);
        var resultBytes = Read(directory, resultFile);

        var result = ResearchArtifactParser.ParseResult(resultBytes, resultFile);
        if (result.ResultId != manifest.Text(entry, "resultId")) throw new ArtifactIntegrityException("result_id_mismatch", resultFile);
        if (result.EvidenceKind != EvidenceKind.Simulated || result.Status != ExperimentResultStatus.Completed
            || result.Policies!.Any(p => p!.RealizedNetProfitQuoteAtoms is not null))
            throw new ArtifactIntegrityException("unsupported_evidence", resultFile);
        if (Convert.ToHexStringLower(SHA256.HashData(requestBytes)) != result.RequestSha256)
            throw new ArtifactIntegrityException("request_hash_mismatch", requestFile);

        var linked = contract switch
        {
            1 => LinkV1(requestBytes, requestFile, result, resultBytes, resultFile, catalog),
            2 => LinkV2(requestBytes, requestFile, result, resultBytes, resultFile, catalog),
            _ => throw manifest.Fail("contract_version_mismatch")
        };
        var quote = catalog.FindDataset(result.DatasetId!)!.Tokens.First(t => t.Id == linked.QuoteTokenId);
        var summary = (contract, summaryFile) switch
        {
            (2, not null) => LoadSummary(Read(directory, summaryFile), summaryFile, result, linked),
            (1, null) => default,
            _ => throw manifest.Fail("summary_mismatch")
        };
        return new(result, contract, Encoding.UTF8.GetString(requestBytes), Encoding.UTF8.GetString(resultBytes),
            quote.Id, quote.Decimals, linked.RouteShapes, linked.TokenIds, linked.PoolCount, linked.Routes,
            summary.Shapes, summary.PoolDirections, summary.ProbeInputAtoms,
            manifest.Text(entry, "sourceCloseout"), Array.AsReadOnly(manifest.Texts(entry, "limitations")));
    }

    sealed record LinkedRequest(string QuoteTokenId, IReadOnlyList<string> RouteShapes, IReadOnlyList<string> TokenIds,
        int PoolCount, IReadOnlyList<ExperimentRouteV2>? Routes);

    static LinkedRequest LinkV1(byte[] bytes, string file, ExperimentResult result, byte[] resultBytes, string resultFile, ResearchCatalog catalog)
    {
        var request = ResearchArtifactParser.ParseRequestV1(bytes, file);
        Reject(ExperimentRequestSyntaxValidator.Validate(request), file);
        Reject(ExperimentResultSyntaxValidator.Validate(result), resultFile);
        RequireCanonical(ExperimentRequestCanonicalizer.Canonicalize(request).Content, bytes, file);
        RequireCanonical(ExperimentResultCanonicalizer.Canonicalize(result).Content, resultBytes, resultFile);
        Reject(ExperimentRequestValidator.Validate(request, catalog), file);
        Reject(ExperimentResultValidator.Validate(result, catalog, request), resultFile);
        return new(request.TokenPair![1]!, ["two_leg"], request.TokenPair.Select(t => t!).ToArray(), request.PoolIds!.Count, null);
    }

    static LinkedRequest LinkV2(byte[] bytes, string file, ExperimentResult result, byte[] resultBytes, string resultFile, ResearchCatalog catalog)
    {
        var request = ResearchArtifactParser.ParseRequestV2(bytes, file);
        Reject(ExperimentRequestV2SyntaxValidator.Validate(request), file);
        Reject(ExperimentResultV2SyntaxValidator.Validate(result), resultFile);
        RequireCanonical(ExperimentRequestV2Canonicalizer.Canonicalize(request).Content, bytes, file);
        RequireCanonical(ExperimentResultV2Canonicalizer.Canonicalize(result).Content, resultBytes, resultFile);
        Reject(ExperimentRequestV2Validator.Validate(request, catalog), file);
        Reject(ExperimentResultValidator.Validate(result, catalog, request), resultFile);
        return new(request.QuoteTokenId!, request.RouteShapes!.Select(s => s!).ToArray(),
            request.TokenIds!.Select(t => t!).ToArray(), request.PoolIds!.Count,
            ExperimentRequestV2RouteEnumerator.Enumerate(request, catalog));
    }

    static (string? Shapes, string? PoolDirections, string? ProbeInputAtoms) LoadSummary(
        byte[] bytes, string file, ExperimentResult result, LinkedRequest request)
    {
        var json = new ArtifactJson(file);
        using var document = json.Parse(bytes);
        var root = document.RootElement;
        json.Exact(root, "classifiedGaps", "droppedForkWrites", "policies", "poolDirections",
            "probeInputAtoms", "routeShapes", "shapes", "slotBoundaries");
        var mismatch = json.Fail("summary_mismatch");
        if (json.Number(root, "routeShapes") != request.Routes!.Count) throw mismatch;

        var shapes = root.GetProperty("shapes");
        var shapeNames = shapes.EnumerateObject().Select(s => s.Name).Order(StringComparer.Ordinal);
        if (!shapeNames.SequenceEqual(request.RouteShapes.Order(StringComparer.Ordinal), StringComparer.Ordinal)) throw mismatch;
        var totals = result.Policies!.ToDictionary(p => p!.Kind!.Value.ToString(), p => Totals.From(p!), StringComparer.Ordinal);

        void Require(IEnumerable<JsonElement> policies)
        {
            var sums = new Dictionary<string, Totals>(StringComparer.Ordinal);
            foreach (var policy in policies)
            {
                var kind = json.Text(policy, "kind");
                sums[kind] = sums.TryGetValue(kind, out var sum) ? sum.Add(Totals.From(json, policy)) : Totals.From(json, policy);
            }
            if (sums.Count != totals.Count || sums.Any(s => !totals.TryGetValue(s.Key, out var t) || !t.SameAs(s.Value))) throw mismatch;
        }
        Require(json.Items(root, "policies"));
        Require(shapes.EnumerateObject().SelectMany(shape => json.Items(shape.Value, "policies")));
        return (shapes.GetRawText(), root.GetProperty("poolDirections").GetRawText(), root.GetProperty("probeInputAtoms").GetRawText());
    }

    sealed record Totals(long Evaluated, long Detected, long Selected, long Estimated, long Simulated, SortedDictionary<string, long> Rejected)
    {
        internal static Totals From(PolicyResult p) => new(p.CandidatesEvaluated!.Value, p.OpportunitiesDetected!.Value,
            p.SelectedOrExecuted!.Value, p.EstimatedNetProfitQuoteAtoms ?? 0, p.SimulatedNetProfitQuoteAtoms ?? 0,
            new(p.RejectedCandidates!.ToDictionary(r => r!.ReasonCode!, r => r!.Count!.Value), StringComparer.Ordinal));

        internal static Totals From(ArtifactJson json, JsonElement p) => new(json.Number(p, "candidatesEvaluated"),
            json.Number(p, "opportunitiesDetected"), json.Number(p, "selectedOrExecuted"),
            json.Number(p, "estimatedNetProfitQuoteAtoms"), json.Number(p, "simulatedNetProfitQuoteAtoms"),
            new(p.GetProperty("rejectedCandidates").EnumerateObject().ToDictionary(r => r.Name, r => r.Value.GetInt64()), StringComparer.Ordinal));

        internal Totals Add(Totals other)
        {
            var rejected = new SortedDictionary<string, long>(Rejected, StringComparer.Ordinal);
            foreach (var (reason, count) in other.Rejected) rejected[reason] = rejected.GetValueOrDefault(reason) + count;
            return new(Evaluated + other.Evaluated, Detected + other.Detected, Selected + other.Selected,
                Estimated + other.Estimated, Simulated + other.Simulated, rejected);
        }

        internal bool SameAs(Totals other) => (Evaluated, Detected, Selected, Estimated, Simulated)
            == (other.Evaluated, other.Detected, other.Selected, other.Estimated, other.Simulated)
            && Rejected.SequenceEqual(other.Rejected);
    }

    static byte[] Read(string directory, string fileName)
    {
        if (fileName.Length == 0 || fileName != Path.GetFileName(fileName) || fileName.Contains('\\'))
            throw new ArtifactIntegrityException("invalid_file_name", fileName);
        try { return File.ReadAllBytes(Path.Combine(directory, fileName)); }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        { throw new ArtifactIntegrityException("missing_file", fileName); }
    }

    static void RequireCanonical(string canonical, byte[] bytes, string file)
    {
        if (!Encoding.UTF8.GetBytes(canonical).AsSpan().SequenceEqual(bytes))
            throw new ArtifactIntegrityException("noncanonical_bytes", file);
    }

    static void Reject(IReadOnlyList<ValidationIssue> issues, string file)
    {
        if (issues.Count != 0) throw new ArtifactIntegrityException("contract_validation_failed", file);
    }
}
