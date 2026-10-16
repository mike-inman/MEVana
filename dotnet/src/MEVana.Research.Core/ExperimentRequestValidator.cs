namespace MEVana.Research.Core;
public static class ExperimentRequestValidator
{
    public static IReadOnlyList<ValidationIssue> Validate(ExperimentRequest? request, ResearchCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        var syntax = ExperimentRequestSyntaxValidator.Validate(request); if (syntax.Count != 0) return syntax;
        var r = request!; var errors = new List<ValidationIssue>();
        void Add(string field, string code) => errors.Add(new(field, code));
        var dataset = catalog.FindDataset(r.DatasetId!);
        if (dataset is null) { Add("datasetId", "unknown_dataset"); return errors.AsReadOnly(); }
        if (!dataset.VerifiedAvailable) Add("datasetId", "dataset_unavailable");
        if (!dataset.SupportedStrategyVersions.Contains(r.StrategyVersion!, StringComparer.Ordinal)) Add("strategyVersion", "unsupported_strategy");
        var pair = r.TokenPair!; var tokens = pair.Select(id => dataset.Tokens.FirstOrDefault(t => t.Id == id)).ToArray();
        if (r.WindowStartUtc < dataset.CoverageStartUtc || r.WindowEndUtc > dataset.CoverageEndUtc) Add("window", "invalid_window");
        if (tokens.Any(t => t is null)) { Add("tokenPair", "incompatible_pair"); return errors.AsReadOnly(); }
            if (r.PoolIds!.Any(id => !dataset.Pools.Any(p => p.Id == id && pair.All(t => p.TokenIds.Contains(t!, StringComparer.Ordinal))))) Add("poolIds", "incompatible_pool");
            var bound = Math.Min(dataset.MaxResearchTradeSizeQuoteAtoms, tokens[1]!.MaxResearchTradeSizeQuoteAtoms);
            if (r.MinTradeSizeQuoteAtoms > bound || r.MaxTradeSizeQuoteAtoms > bound) Add("tradeSizeQuoteAtoms", "invalid_trade_size");

        return errors.AsReadOnly();
    }
}
