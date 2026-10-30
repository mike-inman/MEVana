namespace MEVana.Research.Core;

public sealed record ExperimentRouteV2(string Shape, IReadOnlyList<string> PoolIds, IReadOnlyList<string> TokenIds);

/// <summary>Pure route-universe definition shared by validation and later replay.</summary>
public static class ExperimentRequestV2RouteEnumerator
{
    public static IReadOnlyList<ExperimentRouteV2> Enumerate(ExperimentRequestV2 request, ResearchCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(request);
        ArgumentNullException.ThrowIfNull(catalog);
        if (ExperimentRequestV2Validator.Validate(request, catalog).Count != 0)
            throw new ArgumentException("Invalid typed request or catalog scope.", nameof(request));
        return EnumerateValidMembers(request, catalog.FindDataset(request.DatasetId!)!);
    }

    internal static IReadOnlyList<ExperimentRouteV2> EnumerateValidMembers(ExperimentRequestV2 request, DatasetMetadata dataset)
    {
        var pools = request.PoolIds!.Select(id => dataset.Pools.First(p => p.Id == id))
            .OrderBy(p => p.Id, StringComparer.Ordinal).ToArray();
        var routes = new List<ExperimentRouteV2>();
        string quote = request.QuoteTokenId!;
        if (request.RouteShapes!.Contains("two_leg", StringComparer.Ordinal))
        {
            foreach (var first in pools)
            foreach (var second in pools)
            {
                if (first.Id == second.Id || !first.TokenIds.Contains(quote, StringComparer.Ordinal)) continue;
                var other = first.TokenIds.First(t => t != quote);
                if (!second.TokenIds.Contains(quote, StringComparer.Ordinal)
                    || !second.TokenIds.Contains(other, StringComparer.Ordinal)) continue;
                routes.Add(new("two_leg", Array.AsReadOnly(new[] { first.Id, second.Id }),
                    Array.AsReadOnly(new[] { quote, other, quote })));
            }
        }
        if (request.RouteShapes!.Contains("triangle", StringComparer.Ordinal))
        {
            foreach (var first in pools)
            foreach (var second in pools)
            foreach (var third in pools)
            {
                if (first.Id == second.Id || first.Id == third.Id || second.Id == third.Id
                    || !first.TokenIds.Contains(quote, StringComparer.Ordinal)) continue;
                var middle1 = first.TokenIds.First(t => t != quote);
                if (!second.TokenIds.Contains(middle1, StringComparer.Ordinal)) continue;
                var middle2 = second.TokenIds.First(t => t != middle1);
                if (middle2 == quote || !third.TokenIds.Contains(middle2, StringComparer.Ordinal)
                    || !third.TokenIds.Contains(quote, StringComparer.Ordinal)) continue;
                routes.Add(new("triangle", Array.AsReadOnly(new[] { first.Id, second.Id, third.Id }),
                    Array.AsReadOnly(new[] { quote, middle1, middle2, quote })));
            }
        }
        return routes.OrderBy(r => r.Shape, StringComparer.Ordinal)
            .ThenBy(r => string.Join("\0", r.PoolIds), StringComparer.Ordinal)
            .ThenBy(r => string.Join("\0", r.TokenIds), StringComparer.Ordinal)
            .ToList().AsReadOnly();
    }
}
