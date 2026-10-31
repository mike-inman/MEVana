using System.Globalization;
using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using MEVana.Research.Persistence;

if (args.Length != 2)
{
    Console.Error.WriteLine("Usage: MEVana.Research.SqlExport <seed-directory> <sql-output>");
    return 2;
}
try
{
    var count = SqlExport.WriteValidated(args[0], args[1]);
    Console.WriteLine($"Exported {count} validated experiments.");
    return 0;
}
catch (Exception error) when (error is ArtifactIntegrityException or IOException or UnauthorizedAccessException or JsonException)
{
    Console.Error.WriteLine($"Export refused: {error.GetType().Name}");
    return 1;
}

public static class SqlExport
{
    static readonly HashSet<string> Unevaluable = ["state_gap", "insufficient_state", "tick_array_absent"];
    const string Usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const string Usdt = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
    const string Sol = "So11111111111111111111111111111111111111112";

    static string Symbol(string mint) => mint switch { Usdc => "USDC", Usdt => "USDT", Sol => "SOL", _ => mint };
    public static string Quote(string? text) => text is null ? "NULL" : "'" + text.Replace("'", "''", StringComparison.Ordinal) + "'";
    static string Q(string? text) => Quote(text);
    static string N(long? value) => value?.ToString(CultureInfo.InvariantCulture) ?? "NULL";
    static string J(object value) => Q(JsonSerializer.Serialize(value)) + "::jsonb";
    static string Time(DateTimeOffset? value) => Q(value!.Value.ToUniversalTime().ToString("O", CultureInfo.InvariantCulture)) + "::timestamptz";
    static string Bool(bool value) => value ? "TRUE" : "FALSE";
    static string Array(IEnumerable<long> values) => "ARRAY[" + string.Join(',', values.Select(x => N(x))) + "]::bigint[]";

    public static int WriteValidated(string seedDirectory, string outputFile)
    {
        // Complete validation and SQL generation before opening the output path.
        var store = ResearchArtifactStore.Load(seedDirectory);
        var sql = Build(store);
        File.WriteAllText(outputFile, sql, new UTF8Encoding(false));
        return store.Experiments.Count;
    }

    static string Build(ResearchArtifactStore store)
    {
        var sql = new StringBuilder("-- 2026-10-31: validated research seed load\nBEGIN;\nSET LOCAL standard_conforming_strings = on;\n");
        sql.AppendLine("CREATE TEMP TABLE _research_seed_load_time ON COMMIT DROP AS SELECT CASE WHEN count(*) = 0 THEN CURRENT_TIMESTAMP ELSE min(seed_loaded_at) END AS seed_loaded_at FROM research.experiments;");
        foreach (var table in new[] { "pool_direction_probes", "shape_rejections", "shape_results", "policy_rejections", "policy_results", "experiments" })
            sql.AppendLine($"DELETE FROM research.{table};");
        foreach (var experiment in store.Experiments) ExportExperiment(sql, experiment);
        sql.AppendLine("COMMIT;");
        return sql.ToString();
    }

    static void ExportExperiment(StringBuilder sql, StoredExperiment experiment)
    {
        var r = experiment.Result;
        var id = Q(r.ResultId);
        var quote = experiment.QuoteTokenId;
        var scope = new { routeShapes = experiment.RouteShapes, tokenIds = experiment.TokenIds, poolCount = experiment.PoolCount };
        var scopeLabel = string.Join('/', experiment.TokenIds.Select(Symbol)) +
            (experiment.Routes is null ? "" : $" universe ({experiment.Routes.Count} routes)");
        sql.AppendLine("INSERT INTO research.experiments (result_id,dataset_id,dataset_metadata_version,request_contract_version,evidence_kind,status,window_start_utc,window_end_utc,strategy_version,engine_version,cost_model_version,request_canonicalization_version,request_sha256,quote_mint,quote_symbol,quote_decimals,scope,scope_label,source_closeout,limitations,seed_loaded_at) SELECT " +
            string.Join(',', id, Q(r.DatasetId), Q(r.DatasetMetadataVersion), N(experiment.RequestContractVersion), Q(r.EvidenceKind!.Value.ToString()), Q(r.Status!.Value.ToString()),
                Time(r.WindowStartUtc), Time(r.WindowEndUtc), Q(r.StrategyVersion), Q(r.EngineVersion), Q(r.CostModelVersion),
                N(r.RequestCanonicalizationVersion), Q(r.RequestSha256), Q(quote), Q(Symbol(quote)), N(experiment.QuoteTokenDecimals),
                J(scope), Q(scopeLabel), Q(experiment.SourceCloseout), J(experiment.Limitations)) +
            ",seed_loaded_at FROM _research_seed_load_time;");
        foreach (var policy in r.Policies!) ExportPolicy(sql, id, policy!);
        if (experiment.ShapesJson is not null) ExportShapes(sql, id, experiment.ShapesJson);
        if (experiment.PoolDirectionsJson is not null) ExportProbes(sql, id, experiment.PoolDirectionsJson, experiment.ProbeInputAtomsJson!);
    }

    static void ExportPolicy(StringBuilder sql, string id, PolicyResult policy)
    {
        var kind = Q(policy.Kind!.Value.ToString());
        var missing = policy.RejectedCandidates!.Where(x => Unevaluable.Contains(x!.ReasonCode!)).Sum(x => x!.Count!.Value);
        sql.AppendLine("INSERT INTO research.policy_results VALUES (" + string.Join(',', id, kind,
            N(policy.CandidatesEvaluated), N(policy.OpportunitiesDetected), N(policy.SelectedOrExecuted),
            N(policy.MissedCandidates), N(policy.EstimatedNetProfitQuoteAtoms), N(policy.SimulatedNetProfitQuoteAtoms),
            N(policy.RealizedNetProfitQuoteAtoms), N(missing), N(policy.CandidatesEvaluated!.Value - missing)) + ");");
        foreach (var rejection in policy.RejectedCandidates!)
            sql.AppendLine("INSERT INTO research.policy_rejections VALUES (" + string.Join(',', id, kind,
                Q(rejection!.ReasonCode), N(rejection.Count), Bool(Unevaluable.Contains(rejection.ReasonCode!))) + ");");
    }

    static void ExportShapes(StringBuilder sql, string id, string shapesJson)
    {
        using var document = JsonDocument.Parse(shapesJson);
        foreach (var shape in document.RootElement.EnumerateObject())
        {
            var routeCount = shape.Value.GetProperty("routeShapes").GetInt64();
            foreach (var policy in shape.Value.GetProperty("policies").EnumerateArray())
            {
                var kind = Q(policy.GetProperty("kind").GetString());
                var rejected = policy.GetProperty("rejectedCandidates");
                var missing = rejected.EnumerateObject().Where(x => Unevaluable.Contains(x.Name)).Sum(x => x.Value.GetInt64());
                var evaluated = policy.GetProperty("candidatesEvaluated").GetInt64();
                sql.AppendLine("INSERT INTO research.shape_results VALUES (" + string.Join(',', id, Q(shape.Name), kind,
                    N(routeCount), N(evaluated), N(policy.GetProperty("opportunitiesDetected").GetInt64()),
                    N(policy.GetProperty("selectedOrExecuted").GetInt64()), "NULL",
                    N(policy.GetProperty("estimatedNetProfitQuoteAtoms").GetInt64()),
                    N(policy.GetProperty("simulatedNetProfitQuoteAtoms").GetInt64()), "NULL", N(missing),
                    N(evaluated - missing), Q(policy.GetProperty("simulationUnavailable").GetRawText()) + "::jsonb") + ");");
                foreach (var reason in rejected.EnumerateObject())
                    sql.AppendLine("INSERT INTO research.shape_rejections VALUES (" + string.Join(',', id, Q(shape.Name), kind,
                        Q(reason.Name), N(reason.Value.GetInt64()), Bool(Unevaluable.Contains(reason.Name))) + ");");
            }
        }
    }

    static void ExportProbes(StringBuilder sql, string id, string probesJson, string inputAtomsJson)
    {
        using var probes = JsonDocument.Parse(probesJson);
        using var inputs = JsonDocument.Parse(inputAtomsJson);
        var atoms = Array(inputs.RootElement.EnumerateArray().Select(x => x.GetInt64()));
        foreach (var row in probes.RootElement.EnumerateArray())
            foreach (var outcome in row.GetProperty("outcomes").EnumerateObject())
                sql.AppendLine("INSERT INTO research.pool_direction_probes VALUES (" + string.Join(',', id,
                    Q(row.GetProperty("pool").GetString()), Q(row.GetProperty("inputToken").GetString()),
                    Q(row.GetProperty("outputToken").GetString()), N(row.GetProperty("attempts").GetInt64()),
                    Q(outcome.Name), N(outcome.Value.GetInt64()), atoms) + ");");
    }
}
