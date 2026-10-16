namespace MEVana.Research.Core;
internal static class CatalogRules
{
    internal static string Id(string id) => id is { Length: > 0 and <= 64 } && id.All(c => char.IsAsciiLetterOrDigit(c) || c is '-' or '_') ? id : throw new ArgumentException("Invalid identifier.");
    internal static IReadOnlyList<T> Copy<T>(IReadOnlyList<T> values, int minimum, Func<T, string> key)
    {
        ArgumentNullException.ThrowIfNull(values);
        var copy = values.ToArray();
        if (copy.Length < minimum || copy.Any(v => v is null) || copy.Select(key).Distinct(StringComparer.Ordinal).Count() != copy.Length) throw new ArgumentException("Invalid collection.");
        return Array.AsReadOnly(copy);
    }
    internal static long Positive(long value) => value > 0 ? value : throw new ArgumentException("Positive bound required.");
    internal static string Text(string value) => !string.IsNullOrWhiteSpace(value) && !value.Contains('/') && !value.Contains('\\') && !value.Contains("://") ? value : throw new ArgumentException("Local descriptive text required.");
}
public sealed class TokenMetadata
{
    public string Id { get; }
    public int Decimals { get; }
    public long MaxResearchTradeSizeQuoteAtoms { get; }
    public TokenMetadata(string id, int decimals, long maxResearchTradeSizeQuoteAtoms)
    {
        Id = CatalogRules.Id(id);
        Decimals = decimals is >= 0 and <= 18 ? decimals : throw new ArgumentException("Invalid decimals.");
        MaxResearchTradeSizeQuoteAtoms = CatalogRules.Positive(maxResearchTradeSizeQuoteAtoms);
    }
}
public sealed class PoolMetadata
{
    public string Id { get; }
    public IReadOnlyList<string> TokenIds { get; }
    public PoolMetadata(string id, IReadOnlyList<string> tokenIds)
    {
        Id = CatalogRules.Id(id);
        TokenIds = CatalogRules.Copy(tokenIds, 2, CatalogRules.Id);
        if (TokenIds.Count != 2) throw new ArgumentException("Two token references required.");
        if (TokenIds[0] == TokenIds[1]) throw new ArgumentException("Token IDs must be distinct.");
    }
}
public sealed class DatasetMetadata
{
    public string Id { get; }
    public string MetadataVersion { get; }
    public string Provenance { get; }
    public bool VerifiedAvailable { get; }
    public string Suitability { get; }
    public DateTimeOffset CoverageStartUtc { get; }
    public DateTimeOffset CoverageEndUtc { get; }
    public IReadOnlyList<string> SupportedStrategyVersions { get; }
    public IReadOnlyList<TokenMetadata> Tokens { get; }
    public IReadOnlyList<PoolMetadata> Pools { get; }
    public long MaxResearchTradeSizeQuoteAtoms { get; }
    public DatasetMetadata(string id, string metadataVersion, string provenance, bool verifiedAvailable, string suitability, DateTimeOffset coverageStartUtc, DateTimeOffset coverageEndUtc, IReadOnlyList<string> supportedStrategyVersions, IReadOnlyList<TokenMetadata> tokens, IReadOnlyList<PoolMetadata> pools, long maxResearchTradeSizeQuoteAtoms)
    {
        Id = CatalogRules.Id(id);
        MetadataVersion = CatalogRules.Id(metadataVersion);
        Provenance = CatalogRules.Text(provenance);
        Suitability = CatalogRules.Text(suitability);
        VerifiedAvailable = verifiedAvailable;
        if (coverageStartUtc.Offset != TimeSpan.Zero || coverageEndUtc.Offset != TimeSpan.Zero || coverageStartUtc >= coverageEndUtc) throw new ArgumentException("Invalid UTC coverage.");
        CoverageStartUtc = coverageStartUtc;
        CoverageEndUtc = coverageEndUtc;
        SupportedStrategyVersions = CatalogRules.Copy(supportedStrategyVersions, 1, CatalogRules.Id);
        Tokens = CatalogRules.Copy(tokens, 2, t => t.Id);
        Pools = CatalogRules.Copy(pools, 2, p => p.Id);
        if (Pools.Any(p => p.TokenIds.Any(id => !Tokens.Any(t => t.Id == id)))) throw new ArgumentException("Unknown pool token.");
        MaxResearchTradeSizeQuoteAtoms = CatalogRules.Positive(maxResearchTradeSizeQuoteAtoms);
    }
}
public sealed class ResearchCatalog
{
    public IReadOnlyList<DatasetMetadata> Datasets { get; }
    public static ResearchCatalog Empty { get; } = new(Array.Empty<DatasetMetadata>());
    public ResearchCatalog(IReadOnlyList<DatasetMetadata> datasets) => Datasets = CatalogRules.Copy(datasets, 0, d => d.Id);
    public DatasetMetadata? FindDataset(string id) => Datasets.FirstOrDefault(d => string.Equals(d.Id, id, StringComparison.Ordinal));
}
