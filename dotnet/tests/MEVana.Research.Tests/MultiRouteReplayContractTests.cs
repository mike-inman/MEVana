using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. The route fixture is generated from the .NET v2 enumerator.
public class MultiRouteReplayContractTests
{
    static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-10-30T16:24:16.0436210Z");
    static readonly DateTimeOffset End = DateTimeOffset.Parse("2026-10-30T16:32:14.5720580Z");
    static ExperimentRequestV2 Request() => new("orca-wp-snapshot-20261030", CaptureReplayCatalog.StrategyVersion,
        Start, End, CaptureReplayCatalog.Usdc,
        [CaptureReplayCatalog.Sol, CaptureReplayCatalog.Usdc, CaptureReplayCatalog.Usdt],
        CaptureReplayCatalog.Pools('a').Concat(CaptureReplayCatalog.Pools('b')).Concat(CaptureReplayCatalog.Pools('c')).ToArray(),
        ["two_leg", "triangle"], ["83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d"], 10_000_000, 1_000_000_000);

    [Fact] public void FullOrderedR2FixtureMatchesDotNetEnumerator()
    {
        var routes = ExperimentRequestV2RouteEnumerator.Enumerate(Request(), CaptureReplayCatalog.CreateSnapshot());
        Assert.Equal(152, routes.Count);
        Assert.Equal(120, routes.Count(r => r.Shape == "triangle"));
        Assert.Equal(32, routes.Count(r => r.Shape == "two_leg"));
        var actual = JsonSerializer.SerializeToUtf8Bytes(routes.Select(r => new {shape=r.Shape,pools=r.PoolIds,tokens=r.TokenIds}));
        var fixture = File.ReadAllBytes(Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,
            "../../../../../../crates/research-replay/tests/fixtures/2026-10-30-r2-ordered-routes.json")));
        Assert.Equal(fixture, actual);
    }
}
