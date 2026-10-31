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
