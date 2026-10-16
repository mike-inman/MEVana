using System.Globalization;
using System.Text;
using System.Text.Json;
namespace MEVana.Research.Core;
public sealed record CanonicalRequest(int Version, string Content);
public static class ExperimentRequestCanonicalizer
{
    public static CanonicalRequest Canonicalize(ExperimentRequest request)
    {
        ArgumentNullException.ThrowIfNull(request);
        if (ExperimentRequestSyntaxValidator.Validate(request).Count != 0) throw new ArgumentException("Invalid typed request.", nameof(request));
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject(); writer.WriteNumber("canonicalizationVersion", 1);
            writer.WriteString("datasetId", request.DatasetId); writer.WriteString("strategyVersion", request.StrategyVersion);
            writer.WriteString("windowStartUtc", request.WindowStartUtc!.Value.UtcDateTime.ToString("O", CultureInfo.InvariantCulture));
            writer.WriteString("windowEndUtc", request.WindowEndUtc!.Value.UtcDateTime.ToString("O", CultureInfo.InvariantCulture));
            void Array(string name, IEnumerable<string?> values)
            {
                writer.WriteStartArray(name); foreach (var value in values) writer.WriteStringValue(value); writer.WriteEndArray();
            }
            Array("tokenPair", request.TokenPair!); Array("poolIds", request.PoolIds!.OrderBy(id => id, StringComparer.Ordinal));
            writer.WriteNumber("minTradeSizeQuoteAtoms", request.MinTradeSizeQuoteAtoms!.Value);
            writer.WriteNumber("maxTradeSizeQuoteAtoms", request.MaxTradeSizeQuoteAtoms!.Value);
            writer.WriteEndObject();
        }
        return new(1, Encoding.UTF8.GetString(stream.ToArray()));
    }
}
