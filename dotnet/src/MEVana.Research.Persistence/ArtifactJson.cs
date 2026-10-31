using System.Text.Json;

namespace MEVana.Research.Persistence;

/// <summary>Strict reads: exact property sets, no duplicates, and JavaScript-safe integers only.</summary>
internal sealed class ArtifactJson(string artifact)
{
    const long MaxSafeInteger = 9_007_199_254_740_991;

    internal ArtifactIntegrityException Fail(string code) => new(code, artifact);

    internal JsonDocument Parse(byte[] bytes)
    {
        JsonDocument document;
        try { document = JsonDocument.Parse(bytes); }
        catch (JsonException) { throw Fail("invalid_json"); }
        Inspect(document.RootElement);
        return document;
    }

    void Inspect(JsonElement element)
    {
        switch (element.ValueKind)
        {
            case JsonValueKind.Object:
                var names = new HashSet<string>(StringComparer.Ordinal);
                foreach (var property in element.EnumerateObject())
                {
                    if (!names.Add(property.Name)) throw Fail("duplicate_property");
                    Inspect(property.Value);
                }
                break;
            case JsonValueKind.Array:
                foreach (var item in element.EnumerateArray()) Inspect(item);
                break;
            case JsonValueKind.Number:
                if (!element.TryGetInt64(out var value) || value is > MaxSafeInteger or < -MaxSafeInteger)
                    throw Fail("unsafe_integer");
                break;
        }
    }

    internal void Exact(JsonElement element, params string[] names)
    {
        if (element.ValueKind != JsonValueKind.Object) throw Fail("invalid_value");
        var present = element.EnumerateObject().Select(p => p.Name).ToHashSet(StringComparer.Ordinal);
        if (present.Except(names, StringComparer.Ordinal).Any()) throw Fail("unknown_property");
        if (names.Except(present, StringComparer.Ordinal).Any()) throw Fail("missing_property");
    }

    internal string Text(JsonElement parent, string name) =>
        OptionalText(parent, name) ?? throw Fail("invalid_value");

    internal string? OptionalText(JsonElement parent, string name)
    {
        var value = parent.GetProperty(name);
        return value.ValueKind switch
        {
            JsonValueKind.String => value.GetString(),
            JsonValueKind.Null => null,
            _ => throw Fail("invalid_value")
        };
    }

    internal long Number(JsonElement parent, string name) =>
        OptionalNumber(parent, name) ?? throw Fail("invalid_value");

    internal long? OptionalNumber(JsonElement parent, string name)
    {
        var value = parent.GetProperty(name);
        return value.ValueKind switch
        {
            JsonValueKind.Number => value.GetInt64(),
            JsonValueKind.Null => null,
            _ => throw Fail("invalid_value")
        };
    }

    internal bool Flag(JsonElement parent, string name) => parent.GetProperty(name).ValueKind switch
    {
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => throw Fail("invalid_value")
    };

    internal DateTimeOffset Time(JsonElement parent, string name) =>
        parent.GetProperty(name) is { ValueKind: JsonValueKind.String } value && value.TryGetDateTimeOffset(out var time)
            ? time : throw Fail("invalid_value");

    internal IReadOnlyList<JsonElement> Items(JsonElement parent, string name) =>
        parent.GetProperty(name) is { ValueKind: JsonValueKind.Array } value
            ? value.EnumerateArray().ToArray() : throw Fail("invalid_value");

    internal string[] Texts(JsonElement parent, string name) =>
        Items(parent, name).Select(item => item.ValueKind == JsonValueKind.String
            ? item.GetString()! : throw Fail("invalid_value")).ToArray();

    internal T Named<T>(JsonElement parent, string name) where T : struct, Enum
    {
        var text = Text(parent, name);
        return Enum.GetNames<T>().Contains(text, StringComparer.Ordinal) ? Enum.Parse<T>(text) : throw Fail("invalid_value");
    }
}
