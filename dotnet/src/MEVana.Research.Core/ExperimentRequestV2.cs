namespace MEVana.Research.Core;

/// <summary>Raw immutable multi-pair research request. Invalid submissions remain inspectable.</summary>
public sealed class ExperimentRequestV2
{
    public ExperimentRequestV2(string? datasetId, string? strategyVersion,
        DateTimeOffset? windowStartUtc, DateTimeOffset? windowEndUtc, string? quoteTokenId,
        IReadOnlyList<string?>? tokenIds, IReadOnlyList<string?>? poolIds,
        IReadOnlyList<string?>? routeShapes, IReadOnlyList<string?>? priceInputPoolIds,
        long? minTradeSizeQuoteAtoms, long? maxTradeSizeQuoteAtoms)
    {
        DatasetId = datasetId;
        StrategyVersion = strategyVersion;
        WindowStartUtc = windowStartUtc;
        WindowEndUtc = windowEndUtc;
        QuoteTokenId = quoteTokenId;
        TokenIds = Copy(tokenIds);
        PoolIds = Copy(poolIds);
        RouteShapes = Copy(routeShapes);
        PriceInputPoolIds = Copy(priceInputPoolIds);
        MinTradeSizeQuoteAtoms = minTradeSizeQuoteAtoms;
        MaxTradeSizeQuoteAtoms = maxTradeSizeQuoteAtoms;
    }

    static IReadOnlyList<string?>? Copy(IReadOnlyList<string?>? values) =>
        values is null ? null : Array.AsReadOnly(values.ToArray());

    public string? DatasetId { get; }
    public string? StrategyVersion { get; }
    public DateTimeOffset? WindowStartUtc { get; }
    public DateTimeOffset? WindowEndUtc { get; }
    public string? QuoteTokenId { get; }
    public IReadOnlyList<string?>? TokenIds { get; }
    public IReadOnlyList<string?>? PoolIds { get; }
    public IReadOnlyList<string?>? RouteShapes { get; }
    public IReadOnlyList<string?>? PriceInputPoolIds { get; }
    public long? MinTradeSizeQuoteAtoms { get; }
    public long? MaxTradeSizeQuoteAtoms { get; }
}
