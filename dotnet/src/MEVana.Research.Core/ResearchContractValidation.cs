namespace MEVana.Research.Core;

internal sealed class ContractIssues
{
    readonly List<ValidationIssue> issues = [];
    internal int Count => issues.Count;
    internal void Add(string field, string code) => issues.Add(new(field, code));
    internal IReadOnlyList<ValidationIssue> Finish() => issues.AsReadOnly();
    internal void Id(string field, string? value, bool required = true)
    {
        if (value is null) { if (required) Add(field, "required"); return; }
        try { CatalogRules.Id(value); }
        catch (ArgumentException) { Add(field, "invalid_identifier"); }
    }
    internal void Utc(string field, DateTimeOffset? value, bool required = true)
    {
        if (value is null) { if (required) Add(field, "required"); }
        else if (value.Value.Offset != TimeSpan.Zero) Add(field, "invalid_utc");
    }
    internal void EnumValue<T>(string field, T? value) where T : struct, Enum
    {
        if (value is null) Add(field, "required");
        else if (!Enum.IsDefined(value.Value)) Add(field, "invalid_enum");
    }
    internal void Range(string field, long? value, long min, long max, string code, bool required = true)
    {
        if (value is null) { if (required) Add(field, "required"); }
        else if (value < min || value > max) Add(field, code);
    }
    internal void Schema(int? version)
    {
        if (version is null) Add("schemaVersion", "required");
        else if (version != 1) Add("schemaVersion", "unsupported_schema_version");
    }
    internal void Route(ResearchRoute? route, string field)
    {
        if (route is null) { Add(field, "required"); return; }
        if (route.TokenIds is null) Add(field + ".tokenIds", "required");
        else
        {
            for (int i = 0; i < route.TokenIds.Count; i++) Id(field + $".tokenIds[{i}]", route.TokenIds[i]);
            if (route.TokenIds.Count is < 3 or > 5 || route.TokenIds[0] != route.TokenIds[^1]) Add(field, "invalid_route");
        }
        if (route.PoolIds is null) Add(field + ".poolIds", "required");
        else
        {
            for (int i = 0; i < route.PoolIds.Count; i++) Id(field + $".poolIds[{i}]", route.PoolIds[i]);
            if (route.PoolIds.Count is < 2 or > 4 || route.PoolIds.Distinct(StringComparer.Ordinal).Count() != route.PoolIds.Count
                || route.TokenIds is not null && route.PoolIds.Count != route.TokenIds.Count - 1) Add(field, "invalid_route");
        }
    }
    internal DatasetMetadata? Dataset(ResearchCatalog catalog, string id, string version, string strategy)
    {
        var dataset = catalog.FindDataset(id);
        if (dataset is null) { Add("datasetId", "unknown_dataset"); return null; }
        if (!string.Equals(dataset.MetadataVersion, version, StringComparison.Ordinal)) Add("datasetMetadataVersion", "metadata_version_mismatch");
        if (!dataset.SupportedStrategyVersions.Contains(strategy, StringComparer.Ordinal)) Add("strategyVersion", "unsupported_strategy");
        return dataset;
    }
    internal void CatalogRoute(ResearchRoute route, DatasetMetadata dataset, long size, string field)
    {
        var tokens = route.TokenIds!;
        foreach (var id in tokens.Distinct(StringComparer.Ordinal))
            if (!dataset.Tokens.Any(t => t.Id == id)) Add(field + ".tokenIds", "unknown_token");
        for (int i = 0; i < route.PoolIds!.Count; i++)
        {
            var pool = dataset.Pools.FirstOrDefault(p => p.Id == route.PoolIds[i]);
            if (pool is null) Add(field + $".poolIds[{i}]", "unknown_pool");
            else if (tokens[i] == tokens[i + 1] || !pool.TokenIds.Contains(tokens[i]!, StringComparer.Ordinal)
                || !pool.TokenIds.Contains(tokens[i + 1]!, StringComparer.Ordinal)) Add(field + $".poolIds[{i}]", "incompatible_pool");
        }
        var quote = dataset.Tokens.FirstOrDefault(t => t.Id == tokens[0]);
        if (size > dataset.MaxResearchTradeSizeQuoteAtoms || quote is not null && size > quote.MaxResearchTradeSizeQuoteAtoms)
            Add(field, "invalid_trade_size");
    }
}

public static class ResearchRouteSyntaxValidator
{
    public static IReadOnlyList<ValidationIssue> Validate(ResearchRoute? route)
    {
        var issues = new ContractIssues(); issues.Route(route, "route"); return issues.Finish();
    }
}
