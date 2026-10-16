using MEVana.Research.Core;
using Xunit;
namespace MEVana.Research.Tests;
public class DatasetCatalogContractTests
{
    static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-01-01T00:00:00Z");
    static readonly TokenMetadata[] Tokens = {new("A", 0, 1), new("a", 18, long.MaxValue)};
    static readonly PoolMetadata[] Pools = {new("P", new[] {"A", "a"}), new("p", new[] {"a", "A"})};
    static DatasetMetadata Fixture(string id = "Fixture", string version = "v1", string provenance = "Test fixture", string suitability = "Tests only", DateTimeOffset? start = null, DateTimeOffset? end = null, IReadOnlyList<string>? strategies = null, IReadOnlyList<TokenMetadata>? tokens = null, IReadOnlyList<PoolMetadata>? pools = null, long bound = 1) => new(id, version, provenance, false, suitability, start ?? Start, end ?? Start.AddHours(1), strategies ?? new[] {"S", "s"}, tokens ?? Tokens, pools ?? Pools, bound);

    [Fact]
    public void EmptyLookupAndAvailabilityAreHonest()
    {
        Assert.Empty(ResearchCatalog.Empty.Datasets);
        Assert.Null(ResearchCatalog.Empty.FindDataset("Fixture"));

        var d = Fixture();
        var c = new ResearchCatalog(new[] { d, Fixture("fixture") });
        Assert.Same(d, c.FindDataset("Fixture"));
        Assert.Null(c.FindDataset("FIXTURE"));
        Assert.False(d.VerifiedAvailable);
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData(" ")]
    [InlineData("é")]
    [InlineData("A/B")]
    [InlineData("A.B")]
    public void InvalidIds(string? id) => Assert.ThrowsAny<ArgumentException>(() => new TokenMetadata(id!, 1, 1));

    [Fact]
    public void IdLengthDecimalsAndBounds()
    {
        _ = new TokenMetadata(new string('A', 64), 18, long.MaxValue);
        Assert.ThrowsAny<ArgumentException>(() => new TokenMetadata(new string('A', 65), 1, 1));

        foreach (int n in new[] { -1, 19 })
            Assert.ThrowsAny<ArgumentException>(() => new TokenMetadata("A", n, 1));

        foreach (long n in new[] { 0L, -1L })
        {
            Assert.ThrowsAny<ArgumentException>(() => new TokenMetadata("A", 1, n));
            Assert.ThrowsAny<ArgumentException>(() => Fixture(bound: n));
        }
    }

    [Fact]
    public void RejectInvalidMetadataAndReferences()
    {
        Action[] invalid =
        {
            () => Fixture(version: ""),
            () => Fixture(provenance: " "),
            () => Fixture(suitability: "https://example.test"),
            () => Fixture(start: Start.ToOffset(TimeSpan.FromHours(1))),
            () => Fixture(end: Start),
            () => Fixture(end: Start.AddHours(-1)),
            () => Fixture(strategies: Array.Empty<string>()),
            () => Fixture(strategies: new[] { "S", "S" }),
            () => Fixture(strategies: new string[] { null! }),
            () => Fixture(tokens: new[] { Tokens[0] }),
            () => Fixture(tokens: new[] { Tokens[0], Tokens[0] }),
            () => Fixture(tokens: new TokenMetadata[] { Tokens[0], null! }),
            () => Fixture(pools: new[] { Pools[0] }),
            () => Fixture(pools: new[] { Pools[0], Pools[0] }),
            () => Fixture(pools: new PoolMetadata[] { Pools[0], null! }),
            () => Fixture(pools: new[] { Pools[0], new PoolMetadata("X", new[] { "A", "B" }) }),
            () => new PoolMetadata("P", new[] { "A", "A" }),
            () => new PoolMetadata("P", new[] { "A", null! }),
            () => new PoolMetadata("P", new[] { "A", "a", "B" }),
            () => new PoolMetadata("P", null!),
            () => new ResearchCatalog(null!),
            () => new ResearchCatalog(new DatasetMetadata[] { null! }),
            () => new ResearchCatalog(new[] { Fixture(), Fixture() }),
        };
        foreach (var action in invalid) Assert.ThrowsAny<ArgumentException>(action);
        Assert.ThrowsAny<ArgumentException>(() => new DatasetMetadata("X", "v1", "Fixture", false, "Tests", Start, Start.AddHours(1), null!, Tokens, Pools, 1));
    }

    [Fact]
    public void SnapshotsCannotBeMutated()
    {
        var ids = new[] { "A", "a" };
        var pool = new PoolMetadata("P", ids);
        ids[0] = "X";
        Assert.Equal("A", pool.TokenIds[0]);

        var tokens = Tokens.ToArray();
        var pools = Pools.ToArray();
        var strategies = new[] { "S" };
        var d = Fixture(tokens: tokens, pools: pools, strategies: strategies);
        var datasets = new[] { d };
        var c = new ResearchCatalog(datasets);

        tokens[0] = new TokenMetadata("X", 0, 1);
        pools[0] = new PoolMetadata("X", new[] { "A", "a" });
        strategies[0] = "X";
        datasets[0] = Fixture("X");

        Assert.Equal("A", d.Tokens[0].Id);
        Assert.Equal("P", d.Pools[0].Id);
        Assert.Equal("S", d.SupportedStrategyVersions[0]);
        Assert.Same(d, c.Datasets[0]);

        Assert.Throws<NotSupportedException>(() => ((IList<string>)pool.TokenIds)[0] = "X");
        Assert.Throws<NotSupportedException>(() => ((IList<DatasetMetadata>)c.Datasets)[0] = Fixture("X"));
    }
}
