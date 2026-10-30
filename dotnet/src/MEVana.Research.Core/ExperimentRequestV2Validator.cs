namespace MEVana.Research.Core;

public static class ExperimentRequestV2Validator
{
    public static IReadOnlyList<ValidationIssue> Validate(ExperimentRequestV2? request, ResearchCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        var syntax = ExperimentRequestV2SyntaxValidator.Validate(request);
        if (syntax.Count != 0) return syntax;
        var r = request!;
        var issues = new List<ValidationIssue>();
        void Add(string field, string code) => issues.Add(new(field, code));
        var dataset = catalog.FindDataset(r.DatasetId!);
        if (dataset is null) { Add("datasetId", "unknown_dataset"); return issues.AsReadOnly(); }
        if (!dataset.VerifiedAvailable) Add("datasetId", "dataset_unavailable");
        if (!dataset.SupportedStrategyVersions.Contains(r.StrategyVersion!, StringComparer.Ordinal))
            Add("strategyVersion", "unsupported_strategy");
        if (r.WindowStartUtc < dataset.CoverageStartUtc || r.WindowEndUtc > dataset.CoverageEndUtc)
            Add("window", "invalid_window");
        bool tokensKnown = r.TokenIds!.All(id => dataset.Tokens.Any(t => t.Id == id));
        if (!tokensKnown) Add("tokenIds", "incompatible_token");
        var selected = r.PoolIds!.Select(id => dataset.Pools.FirstOrDefault(p => p.Id == id)).ToArray();
        bool legsKnown = selected.All(p => p is not null && p.TokenIds.All(t => r.TokenIds!.Contains(t, StringComparer.Ordinal)));
        if (!legsKnown) Add("poolIds", "incompatible_pool");
        if (r.PriceInputPoolIds!.Any(id => !dataset.Pools.Any(p => p.Id == id)))
            Add("priceInputPoolIds", "incompatible_pool");
        if (tokensKnown)
        {
            var quote = dataset.Tokens.First(t => t.Id == r.QuoteTokenId);
            var bound = Math.Min(dataset.MaxResearchTradeSizeQuoteAtoms, quote.MaxResearchTradeSizeQuoteAtoms);
            if (r.MinTradeSizeQuoteAtoms > bound || r.MaxTradeSizeQuoteAtoms > bound)
                Add("tradeSizeQuoteAtoms", "invalid_trade_size");
        }
        if (tokensKnown && legsKnown)
        {
            var routes = ExperimentRequestV2RouteEnumerator.EnumerateValidMembers(r, dataset);
            foreach (var shape in r.RouteShapes!)
                if (!routes.Any(route => route.Shape == shape)) Add("routeShapes", "shape_unsatisfiable");
            foreach (var pool in r.PoolIds!)
                if (!routes.Any(route => route.PoolIds.Contains(pool!, StringComparer.Ordinal)))
                    Add("poolIds", "unused_pool");
        }
        return issues.AsReadOnly();
    }
}
