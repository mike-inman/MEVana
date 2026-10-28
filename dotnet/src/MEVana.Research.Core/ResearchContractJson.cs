using System.Globalization;
using System.Text;
using System.Text.Json;

namespace MEVana.Research.Core;

internal static class ResearchContractJson
{
    internal static string Write(Action<Utf8JsonWriter> write)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream)) write(writer);
        return Encoding.UTF8.GetString(stream.ToArray());
    }
    internal static void Number(Utf8JsonWriter writer, string field, long? value)
    {
        if (value is { } number) writer.WriteNumber(field, number); else writer.WriteNull(field);
    }
    internal static void Unsigned(Utf8JsonWriter writer, string field, ulong? value)
    {
        if (value is { } number) writer.WriteNumber(field, number); else writer.WriteNull(field);
    }
    internal static void Time(Utf8JsonWriter writer, string field, DateTimeOffset? value)
    {
        if (value is { } time) writer.WriteString(field, time.UtcDateTime.ToString("O", CultureInfo.InvariantCulture));
        else writer.WriteNull(field);
    }
    internal static void Route(Utf8JsonWriter writer, ResearchRoute route)
    {
        writer.WriteStartObject("route");
        writer.WriteStartArray("tokenIds"); foreach (var token in route.TokenIds!) writer.WriteStringValue(token); writer.WriteEndArray();
        writer.WriteStartArray("poolIds"); foreach (var pool in route.PoolIds!) writer.WriteStringValue(pool); writer.WriteEndArray();
        writer.WriteEndObject();
    }
}
