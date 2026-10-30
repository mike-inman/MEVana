namespace MEVana.Research.Core;

public static class ExperimentRequestV2SyntaxValidator
{
    static bool Id(string? value) => value is { Length: > 0 and <= 64 }
        && value.All(c => char.IsAsciiLetterOrDigit(c) || c is '-' or '_');

    public static IReadOnlyList<ValidationIssue> Validate(ExperimentRequestV2? request)
    {
        var issues = new List<ValidationIssue>();
        void Add(string field, string code) => issues.Add(new(field, code));
        if (request is null) { Add("request", "required"); return issues.AsReadOnly(); }
        void Identifier(string field, string? value)
        {
            if (value is null) Add(field, "required");
            else if (!Id(value)) Add(field, "invalid_identifier");
        }
        bool List(string field, IReadOnlyList<string?>? values, int min, int max, string code)
        {
            if (values is null) { Add(field, "required"); return false; }
            if (values.Count < min || values.Count > max || values.Any(v => !Id(v))
                || values.Distinct(StringComparer.Ordinal).Count() != values.Count)
            { Add(field, code); return false; }
            return true;
        }
        Identifier("datasetId", request.DatasetId);
        Identifier("strategyVersion", request.StrategyVersion);
        if (request.WindowStartUtc is null) Add("windowStartUtc", "required");
        if (request.WindowEndUtc is null) Add("windowEndUtc", "required");
        if (request.WindowStartUtc is { } start && request.WindowEndUtc is { } end
            && (start >= end || end - start > TimeSpan.FromHours(24))) Add("window", "invalid_window");
        Identifier("quoteTokenId", request.QuoteTokenId);
        if (List("tokenIds", request.TokenIds, 2, 8, "invalid_token_ids")
            && Id(request.QuoteTokenId)
            && !request.TokenIds!.Contains(request.QuoteTokenId, StringComparer.Ordinal))
            Add("quoteTokenId", "invalid_quote_token");
        List("poolIds", request.PoolIds, 2, 32, "invalid_pool_ids");
        if (List("routeShapes", request.RouteShapes, 1, 2, "invalid_route_shapes")
            && request.RouteShapes!.Any(s => s is not ("two_leg" or "triangle")))
            Add("routeShapes", "invalid_route_shapes");
        List("priceInputPoolIds", request.PriceInputPoolIds, 0, 4, "invalid_price_input_pools");
        if (request.MinTradeSizeQuoteAtoms is null) Add("minTradeSizeQuoteAtoms", "required");
        if (request.MaxTradeSizeQuoteAtoms is null) Add("maxTradeSizeQuoteAtoms", "required");
        if (request.MinTradeSizeQuoteAtoms is { } min && request.MaxTradeSizeQuoteAtoms is { } max
            && (min <= 0 || max <= 0 || min > max)) Add("tradeSizeQuoteAtoms", "invalid_trade_size");
        return issues.AsReadOnly();
    }
}
