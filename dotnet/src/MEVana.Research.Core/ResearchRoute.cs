namespace MEVana.Research.Core;

/// <summary>Raw immutable research submission; validate before consumption.</summary>
public sealed class ResearchRoute
{
    public ResearchRoute(IReadOnlyList<string?>? tokenIds,
        IReadOnlyList<string?>? poolIds)
    {
        TokenIds = tokenIds is null ? null : Array.AsReadOnly(tokenIds.ToArray());
        PoolIds = poolIds is null ? null : Array.AsReadOnly(poolIds.ToArray());
    }

    public IReadOnlyList<string?>? TokenIds { get; }
    public IReadOnlyList<string?>? PoolIds { get; }
}
