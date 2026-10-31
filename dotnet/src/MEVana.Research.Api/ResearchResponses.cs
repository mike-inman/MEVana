using System.Globalization;
using System.Text.Json;
using MEVana.Research.Core;
using MEVana.Research.Persistence;

namespace MEVana.Research.Api;

/// <summary>Response bytes built once at startup, so repeated requests are byte-identical.</summary>
public sealed class ResearchResponses
{
    static readonly string[] UnevaluableReasons = ["state_gap", "insufficient_state", "tick_array_absent"];

    public byte[] DatasetList { get; }
    public IReadOnlyDictionary<string, byte[]> Datasets { get; }
    public byte[] ExperimentList { get; }
    public IReadOnlyDictionary<string, byte[]> Experiments { get; }

    public ResearchResponses(ResearchArtifactStore store)
    {
        DatasetList = Build(w =>
        {
            w.WriteStartObject();
            w.WriteStartArray("datasets");
            foreach (var dataset in store.Catalog.Datasets) WriteDataset(w, dataset);
            w.WriteEndArray();
            w.WriteEndObject();
        });
        Datasets = store.Catalog.Datasets.ToDictionary(d => d.Id, d => Build(w => WriteDataset(w, d)), StringComparer.Ordinal);
        ExperimentList = Build(w =>
        {
            w.WriteStartObject();
            w.WriteStartArray("experiments");
            foreach (var experiment in store.Experiments)
            {
                w.WriteStartObject();
                WriteSummary(w, experiment);
                w.WriteEndObject();
            }
            w.WriteEndArray();
            w.WriteEndObject();
        });
        Experiments = store.Experiments.ToDictionary(e => e.Result.ResultId!, e => Build(w => WriteDetail(w, e)), StringComparer.Ordinal);
    }

    static byte[] Build(Action<Utf8JsonWriter> write)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream)) write(writer);
        return stream.ToArray();
    }

    static string Utc(DateTimeOffset? value) => value!.Value.UtcDateTime.ToString("O", CultureInfo.InvariantCulture);

    static void Number(Utf8JsonWriter w, string name, long? value)
    {
        if (value is { } number) w.WriteNumber(name, number);
        else w.WriteNull(name);
    }

    static void Strings(Utf8JsonWriter w, string name, IEnumerable<string> values)
    {
        w.WriteStartArray(name);
        foreach (var value in values) w.WriteStringValue(value);
        w.WriteEndArray();
    }

    static void WriteDataset(Utf8JsonWriter w, DatasetMetadata d)
    {
        w.WriteStartObject();
        w.WriteString("id", d.Id);
        w.WriteString("metadataVersion", d.MetadataVersion);
        w.WriteString("provenance", d.Provenance);
        w.WriteBoolean("verifiedAvailable", d.VerifiedAvailable);
        w.WriteString("suitability", d.Suitability);
        w.WriteString("coverageStartUtc", Utc(d.CoverageStartUtc));
        w.WriteString("coverageEndUtc", Utc(d.CoverageEndUtc));
        Strings(w, "supportedStrategyVersions", d.SupportedStrategyVersions);
        w.WriteStartArray("tokens");
        foreach (var token in d.Tokens)
        {
            w.WriteStartObject();
            w.WriteString("id", token.Id);
            w.WriteNumber("decimals", token.Decimals);
            w.WriteNumber("maxResearchTradeSizeQuoteAtoms", token.MaxResearchTradeSizeQuoteAtoms);
            w.WriteEndObject();
        }
        w.WriteEndArray();
        w.WriteStartArray("pools");
        foreach (var pool in d.Pools)
        {
            w.WriteStartObject();
            w.WriteString("id", pool.Id);
            Strings(w, "tokenIds", pool.TokenIds);
            w.WriteEndObject();
        }
        w.WriteEndArray();
        w.WriteNumber("maxResearchTradeSizeQuoteAtoms", d.MaxResearchTradeSizeQuoteAtoms);
        w.WriteEndObject();
    }

    static void WriteSummary(Utf8JsonWriter w, StoredExperiment e)
    {
        var r = e.Result;
        w.WriteString("resultId", r.ResultId);
        w.WriteString("datasetId", r.DatasetId);
        w.WriteString("strategyVersion", r.StrategyVersion);
        w.WriteString("engineVersion", r.EngineVersion);
        w.WriteString("costModelVersion", r.CostModelVersion);
        w.WriteNumber("requestCanonicalizationVersion", r.RequestCanonicalizationVersion!.Value);
        w.WriteString("requestSha256", r.RequestSha256);
        w.WriteString("windowStartUtc", Utc(r.WindowStartUtc));
        w.WriteString("windowEndUtc", Utc(r.WindowEndUtc));
        w.WriteString("evidenceKind", r.EvidenceKind!.Value.ToString());
        w.WriteString("status", r.Status!.Value.ToString());
        w.WriteString("quoteTokenId", e.QuoteTokenId);
        w.WriteNumber("quoteTokenDecimals", e.QuoteTokenDecimals);
        w.WriteStartObject("scope");
        Strings(w, "routeShapes", e.RouteShapes);
        Strings(w, "tokenIds", e.TokenIds);
        w.WriteNumber("poolCount", e.PoolCount);
        w.WriteEndObject();
        w.WriteStartArray("policies");
        foreach (var policy in r.Policies!) WritePolicy(w, policy!);
        w.WriteEndArray();
        w.WriteStartArray("coverage");
        foreach (var policy in r.Policies!) WriteCoverage(w, policy!);
        w.WriteEndArray();
    }

    static void WritePolicy(Utf8JsonWriter w, PolicyResult p)
    {
        w.WriteStartObject();
        w.WriteString("kind", p.Kind!.Value.ToString());
        Number(w, "candidatesEvaluated", p.CandidatesEvaluated);
        Number(w, "opportunitiesDetected", p.OpportunitiesDetected);
        Number(w, "selectedOrExecuted", p.SelectedOrExecuted);
        w.WriteStartArray("rejectedCandidates");
        foreach (var rejection in p.RejectedCandidates!)
        {
            w.WriteStartObject();
            w.WriteString("reasonCode", rejection!.ReasonCode);
            Number(w, "count", rejection.Count);
            w.WriteEndObject();
        }
        w.WriteEndArray();
        Number(w, "missedCandidates", p.MissedCandidates);
        Number(w, "estimatedNetProfitQuoteAtoms", p.EstimatedNetProfitQuoteAtoms);
        Number(w, "simulatedNetProfitQuoteAtoms", p.SimulatedNetProfitQuoteAtoms);
        Number(w, "realizedNetProfitQuoteAtoms", p.RealizedNetProfitQuoteAtoms);
        w.WriteEndObject();
    }

    static void WriteCoverage(Utf8JsonWriter w, PolicyResult p)
    {
        var evaluated = p.CandidatesEvaluated!.Value;
        var unevaluable = p.RejectedCandidates!.Where(r => UnevaluableReasons.Contains(r!.ReasonCode, StringComparer.Ordinal))
            .Sum(r => r!.Count!.Value);
        w.WriteStartObject();
        w.WriteString("kind", p.Kind!.Value.ToString());
        w.WriteNumber("candidatesEvaluated", evaluated);
        w.WriteNumber("unevaluableCandidates", unevaluable);
        w.WriteNumber("evaluableCandidates", evaluated - unevaluable);
        w.WriteEndObject();
    }

    static void WriteDetail(Utf8JsonWriter w, StoredExperiment e)
    {
        w.WriteStartObject();
        WriteSummary(w, e);
        // Written unchanged so a client can recompute requestSha256 over the served value.
        w.WritePropertyName("request");
        w.WriteRawValue(e.RequestJson, skipInputValidation: true);
        w.WritePropertyName("result");
        w.WriteRawValue(e.ResultJson, skipInputValidation: true);
        if (e.Routes is null) w.WriteNull("routeCounts");
        else
        {
            w.WriteStartObject("routeCounts");
            foreach (var shape in e.Routes.GroupBy(r => r.Shape).OrderBy(g => g.Key, StringComparer.Ordinal))
                w.WriteNumber(shape.Key, shape.Count());
            w.WriteNumber("total", e.Routes.Count);
            w.WriteEndObject();
        }
        w.WritePropertyName("shapes");
        if (e.ShapesJson is null) w.WriteNullValue();
        else w.WriteRawValue(e.ShapesJson, skipInputValidation: true);
        w.WritePropertyName("poolDirections");
        if (e.PoolDirectionsJson is null) w.WriteNullValue();
        else
        {
            w.WriteStartObject();
            w.WriteBoolean("diagnosticProbe", true);
            w.WritePropertyName("probeInputAtoms");
            w.WriteRawValue(e.ProbeInputAtomsJson!, skipInputValidation: true);
            w.WritePropertyName("rows");
            w.WriteRawValue(e.PoolDirectionsJson, skipInputValidation: true);
            w.WriteEndObject();
        }
        Strings(w, "limitations", e.Limitations);
        w.WriteString("sourceCloseout", e.SourceCloseout);
        w.WriteEndObject();
    }
}
