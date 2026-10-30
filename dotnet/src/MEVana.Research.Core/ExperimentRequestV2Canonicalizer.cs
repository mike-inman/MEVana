using System.Globalization;
using System.Text;
using System.Text.Json;

namespace MEVana.Research.Core;

public static class ExperimentRequestV2Canonicalizer
{
    public static CanonicalRequest Canonicalize(ExperimentRequestV2 request)
    {
        ArgumentNullException.ThrowIfNull(request);
        if (ExperimentRequestV2SyntaxValidator.Validate(request).Count != 0)
            throw new ArgumentException("Invalid typed request.", nameof(request));
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            writer.WriteNumber("canonicalizationVersion", 2);
            writer.WriteString("datasetId", request.DatasetId);
            writer.WriteString("strategyVersion", request.StrategyVersion);
            writer.WriteString("windowStartUtc", request.WindowStartUtc!.Value.UtcDateTime.ToString("O", CultureInfo.InvariantCulture));
            writer.WriteString("windowEndUtc", request.WindowEndUtc!.Value.UtcDateTime.ToString("O", CultureInfo.InvariantCulture));
            writer.WriteString("quoteTokenId", request.QuoteTokenId);
            void Array(string name, IReadOnlyList<string?> values)
            {
                writer.WriteStartArray(name);
                foreach (var value in values.OrderBy(v => v, StringComparer.Ordinal)) writer.WriteStringValue(value);
                writer.WriteEndArray();
            }
            Array("tokenIds", request.TokenIds!);
            Array("poolIds", request.PoolIds!);
            Array("routeShapes", request.RouteShapes!);
            Array("priceInputPoolIds", request.PriceInputPoolIds!);
            writer.WriteNumber("minTradeSizeQuoteAtoms", request.MinTradeSizeQuoteAtoms!.Value);
            writer.WriteNumber("maxTradeSizeQuoteAtoms", request.MaxTradeSizeQuoteAtoms!.Value);
            writer.WriteEndObject();
        }
        return new(2, Encoding.UTF8.GetString(stream.ToArray()));
    }
}
