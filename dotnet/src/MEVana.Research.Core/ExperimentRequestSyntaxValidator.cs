namespace MEVana.Research.Core;
public sealed record ValidationIssue(string Field, string Code);
public static class ExperimentRequestSyntaxValidator
{
    static bool Id(string? id) => id is { Length: > 0 and <= 64 } && id.All(c => char.IsAsciiLetterOrDigit(c) || c is '-' or '_');
    public static IReadOnlyList<ValidationIssue> Validate(ExperimentRequest? request)
    {
        var errors = new List<ValidationIssue>();
        void Add(string field, string code) => errors.Add(new(field, code));
        if (request is null) { Add("request", "required"); return errors.AsReadOnly(); }
        void Identifier(string field, string? value) { if (value is null) Add(field, "required"); else if (!Id(value)) Add(field, "invalid_identifier"); }
        void List(string field, IReadOnlyList<string?>? values, int min, int max, string code)
        {
            if (values is null) Add(field, "required");
            else if (values.Count < min || values.Count > max || values.Any(v => !Id(v)) || values.Distinct(StringComparer.Ordinal).Count() != values.Count) Add(field, code);
        }
        Identifier("datasetId", request.DatasetId); Identifier("strategyVersion", request.StrategyVersion);
        if (request.WindowStartUtc is null) Add("windowStartUtc", "required");
        if (request.WindowEndUtc is null) Add("windowEndUtc", "required");
        if (request.WindowStartUtc is { } start && request.WindowEndUtc is { } end && (start >= end || end - start > TimeSpan.FromHours(24))) Add("window", "invalid_window");
        List("tokenPair", request.TokenPair, 2, 2, "invalid_token_pair"); List("poolIds", request.PoolIds, 2, 16, "invalid_pool_ids");
        if (request.MinTradeSizeQuoteAtoms is null) Add("minTradeSizeQuoteAtoms", "required");
        if (request.MaxTradeSizeQuoteAtoms is null) Add("maxTradeSizeQuoteAtoms", "required");
        if (request.MinTradeSizeQuoteAtoms is { } min && request.MaxTradeSizeQuoteAtoms is { } max && (min <= 0 || max <= 0 || min > max)) Add("tradeSizeQuoteAtoms", "invalid_trade_size");
        return errors.AsReadOnly();
    }
}
