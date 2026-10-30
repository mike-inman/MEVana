using MEVana.Research.Core;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. Bounded, public Processed dev/test capture; no provider access.
internal static class CaptureReplayCatalog
{
    internal const string DatasetId = "orca-wp-allowlist-20261029";
    internal const string StrategyVersion = "ranking-v0-heuristic";
    internal const string Sol = "So11111111111111111111111111111111111111112";
    internal const string Usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    internal const string Usdt = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
    static readonly string[] PairA = [
        "21gTfxAnhUDjJGZJDkTXctGFKT8TeiXx6pN1CEg9K1uW", "7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm",
        "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d", "BSddxwYW73as8852ZTHRH13pbZEmZ96NBjayc5mSVtkZ",
        "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE", "FpCMFDFGYotvufJ7HrFHsWEiiQCGbkLCtwHiDnh7o28Q"];
    static readonly string[] PairB = [
        "AhhoxZDmsg2snm85vPjqzYzEYESoKfb4KmTj4HrBBNwY", "B6LL9aCWVuo1tTcJoYvCTDqYrq1vjMfci8uHxsm4UxTR",
        "BmXVhzBHTawt5stWKBtFdjM8n33G5RoUDECE5FrRvayq", "FwewVm8u6tFPGewAyHmWAqad9hmF7mvqxK4mJ7iNqqGC",
        "HcoJqG325TTifs6jyWvRJ9ET4pDu12Xrt2EQKZGFmuKX"];
    static readonly string[] PairC = ["4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4", "ELqXddRqcnRJXQnqZdPwBXEEBpQ6SEbCQrBe12wpHBU"];

    internal static IReadOnlyList<string> Pools(char pair) => pair switch
    {
        'a' => PairA, 'b' => PairB, 'c' => PairC, _ => throw new ArgumentOutOfRangeException(nameof(pair))
    };

    internal static ResearchCatalog Create()
    {
        const long bound = 1_000_000_000;
        var start = DateTimeOffset.Parse("2026-10-30T03:21:31.3405290Z");
        var end = DateTimeOffset.Parse("2026-10-30T03:31:00.5213970Z");
        var tokens = new[] { new TokenMetadata(Sol, 9, bound), new TokenMetadata(Usdc, 6, bound), new TokenMetadata(Usdt, 6, bound) };
        var pools = PairA.Select(id => new PoolMetadata(id, [Sol, Usdc]))
            .Concat(PairB.Select(id => new PoolMetadata(id, [Sol, Usdt])))
            .Concat(PairC.Select(id => new PoolMetadata(id, [Usdc, Usdt]))).ToArray();
        return new ResearchCatalog([new DatasetMetadata(DatasetId, "capture-v1",
            "capture-1793330490539629000-dev-test-processed", true, "Simulated research only",
            start, end, [StrategyVersion], tokens, pools, bound)]);
    }
}
