using System.Globalization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MEVana.Research.Core;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. Contract-only examples based on the public handoff 09 allowlist.
public class ExperimentRequestV2Tests
{
    static readonly DateTimeOffset Start = DateTimeOffset.Parse("2026-10-30T03:22:00Z");
    static readonly DateTimeOffset End = DateTimeOffset.Parse("2026-10-30T03:30:00Z");
    static string[] AllPools() => CaptureReplayCatalog.Pools('a').Concat(CaptureReplayCatalog.Pools('b'))
        .Concat(CaptureReplayCatalog.Pools('c')).ToArray();
    static ExperimentRequestV2 Request(string? dataset = CaptureReplayCatalog.DatasetId,
        string? strategy = CaptureReplayCatalog.StrategyVersion, string? quote = CaptureReplayCatalog.Usdc,
        IReadOnlyList<string?>? tokens = null, IReadOnlyList<string?>? pools = null,
        IReadOnlyList<string?>? shapes = null, IReadOnlyList<string?>? price = null,
        long? min = 1, long? max = 1_000_000_000,
        DateTimeOffset? start = null, DateTimeOffset? end = null) =>
        new(dataset, strategy, start ?? Start, end ?? End, quote,
            tokens ?? [CaptureReplayCatalog.Sol, CaptureReplayCatalog.Usdc, CaptureReplayCatalog.Usdt],
            pools ?? AllPools(), shapes ?? ["triangle"], price ?? [], min, max);
    static ExperimentRequestV2 R1() => Request();
    static ExperimentRequestV2 R2() => Request(shapes: ["two_leg", "triangle"]);
    static ExperimentRequestV2 R3() => Request(pools: CaptureReplayCatalog.Pools('a')
        .Concat(CaptureReplayCatalog.Pools('b').Take(1)).ToArray(), shapes: ["two_leg"]);
    static void Syntax(ExperimentRequestV2 r, string field, string code) =>
        Assert.Contains(new ValidationIssue(field, code), ExperimentRequestV2SyntaxValidator.Validate(r));
    static void Catalog(ExperimentRequestV2 r, ResearchCatalog c, string field, string code) =>
        Assert.Contains(new ValidationIssue(field, code), ExperimentRequestV2Validator.Validate(r, c));

    [Fact] public void RawPreservationAndSyntaxBoundariesMirrorV1()
    {
        Assert.Equal(new ValidationIssue("request", "required"), Assert.Single(ExperimentRequestV2SyntaxValidator.Validate(null)));
        var missing = new ExperimentRequestV2(null, null, null, null, null, null, null, null, null, null, null);
        Assert.Equal(11, ExperimentRequestV2SyntaxValidator.Validate(missing).Count);
        var tokens = new string?[] { "A", null, "A" }; var pools = new string?[] { "p", "p" };
        var raw = Request(tokens: tokens, pools: pools, shapes: [null], price: ["x", "x"], min: -1);
        tokens[0] = "changed"; pools[0] = "changed";
        Assert.Equal("A", raw.TokenIds![0]); Assert.Equal("p", raw.PoolIds![0]);
        Assert.Throws<NotSupportedException>(() => ((IList<string?>)raw.TokenIds).Clear());
        Assert.Equal(raw.TokenIds, Request(tokens: ["A", null, "A"]).TokenIds);
        var issues = ExperimentRequestV2SyntaxValidator.Validate(raw);
        Assert.Equal(issues, ExperimentRequestV2SyntaxValidator.Validate(raw));
        Assert.Throws<NotSupportedException>(() => ((IList<ValidationIssue>)issues).Clear());
        foreach (var id in new[] { "", " ", "é", "a/b", new string('x', 65) })
        { Syntax(Request(dataset: id), "datasetId", "invalid_identifier"); Syntax(Request(strategy: id), "strategyVersion", "invalid_identifier"); }
        Syntax(Request(start: End), "window", "invalid_window");
        Syntax(Request(end: Start.AddHours(24).AddTicks(1)), "window", "invalid_window");
        Syntax(Request(min: 0), "tradeSizeQuoteAtoms", "invalid_trade_size");
        Syntax(Request(min: 2, max: 1), "tradeSizeQuoteAtoms", "invalid_trade_size");
        Assert.Empty(ExperimentRequestV2SyntaxValidator.Validate(Request(
            tokens: Enumerable.Range(0, 8).Select(i => (string?)$"T{i}").Append(CaptureReplayCatalog.Usdc).Take(8).ToArray(),
            quote: "T0", pools: Enumerable.Range(0, 32).Select(i => (string?)$"P{i}").ToArray(),
            price: ["P0", "P1", "P2", "P3"], min: long.MaxValue, max: long.MaxValue,
            end: Start.AddHours(24), start: Start.ToOffset(TimeSpan.FromHours(2)))));
    }

    [Fact] public void V2SpecificSyntaxCodes()
    {
        foreach (var tokens in new string?[][] { [], ["A"], ["A", "A"], ["A", null], Enumerable.Range(0, 9).Select(i => (string?)$"T{i}").ToArray() })
            Syntax(Request(tokens: tokens), "tokenIds", "invalid_token_ids");
        Syntax(Request(quote: "Other"), "quoteTokenId", "invalid_quote_token");
        foreach (var pools in new string?[][] { [], ["P"], ["P", "P"], ["P", null], Enumerable.Range(0, 33).Select(i => (string?)$"P{i}").ToArray() })
            Syntax(Request(pools: pools), "poolIds", "invalid_pool_ids");
        foreach (var shapes in new string?[][] { [], ["two_leg", "two_leg"], ["other"], ["triangle", "two_leg", "other"] })
            Syntax(Request(shapes: shapes), "routeShapes", "invalid_route_shapes");
        foreach (var prices in new string?[][] { ["P", "P"], [null], Enumerable.Range(0, 5).Select(i => (string?)$"P{i}").ToArray() })
            Syntax(Request(price: prices), "priceInputPoolIds", "invalid_price_input_pools");
        Assert.Empty(ExperimentRequestV2SyntaxValidator.Validate(Request(
            tokens: ["A", "a"], quote: "A", pools: ["P", "p"], shapes: ["two_leg"], price: [])));
    }

    [Fact] public void CatalogRulesAndRouteExamples()
    {
        var catalog = CaptureReplayCatalog.Create();
        Assert.Empty(ExperimentRequestV2Validator.Validate(R1(), catalog));
        Assert.Equal(120, ExperimentRequestV2RouteEnumerator.Enumerate(R1(), catalog).Count);
        var routes = ExperimentRequestV2RouteEnumerator.Enumerate(R2(), catalog);
        Assert.Empty(ExperimentRequestV2Validator.Validate(R2(), catalog));
        Assert.Equal(152, routes.Count);
        Assert.Equal(120, routes.Count(r => r.Shape == "triangle"));
        Assert.Equal(32, routes.Count(r => r.Shape == "two_leg"));
        Assert.All(routes, r => { Assert.Equal(CaptureReplayCatalog.Usdc, r.TokenIds[0]); Assert.Equal(CaptureReplayCatalog.Usdc, r.TokenIds[^1]); Assert.Equal(r.PoolIds.Count + 1, r.TokenIds.Count); Assert.Equal(r.PoolIds.Count, r.PoolIds.Distinct().Count()); });
        Assert.Equal(routes, ExperimentRequestV2RouteEnumerator.Enumerate(R2(), catalog));
        Catalog(R3(), catalog, "poolIds", "unused_pool");
        Assert.Throws<ArgumentException>(() => ExperimentRequestV2RouteEnumerator.Enumerate(R3(), catalog));
        Catalog(Request(shapes: ["triangle"], pools: CaptureReplayCatalog.Pools('a')), catalog, "routeShapes", "shape_unsatisfiable");
        Catalog(Request(dataset: "missing"), catalog, "datasetId", "unknown_dataset");
        Catalog(Request(dataset: CaptureReplayCatalog.DatasetId.ToLowerInvariant()), catalog, "datasetId", "unknown_dataset");
        Catalog(Request(strategy: "other"), catalog, "strategyVersion", "unsupported_strategy");
        Catalog(Request(start: Start.AddTicks(-1).AddMinutes(-1)), catalog, "window", "invalid_window");
        Catalog(Request(tokens: [CaptureReplayCatalog.Usdc, "Unknown"], pools: ["A", "B"]), catalog, "tokenIds", "incompatible_token");
        Catalog(Request(pools: ["Unknown", CaptureReplayCatalog.Pools('a')[0]]), catalog, "poolIds", "incompatible_pool");
        Catalog(Request(price: ["Unknown"]), catalog, "priceInputPoolIds", "incompatible_pool");
        Catalog(Request(tokens: [CaptureReplayCatalog.Usdc, CaptureReplayCatalog.Sol], price: [CaptureReplayCatalog.Pools('b')[0]]), catalog, "poolIds", "incompatible_pool");
        Catalog(Request(max: 1_000_000_001), catalog, "tradeSizeQuoteAtoms", "invalid_trade_size");
        Assert.Throws<ArgumentNullException>(() => ExperimentRequestV2Validator.Validate(R1(), null!));
        Assert.Equal(ExperimentRequestV2SyntaxValidator.Validate(null), ExperimentRequestV2Validator.Validate(null, catalog));
    }

    [Theory] [InlineData("r1")] [InlineData("r2")] [InlineData("r3")]
    public void GoldenCanonicalBytesAndSha(string name)
    {
        var request = name switch { "r1" => R1(), "r2" => R2(), _ => R3() };
        var bytes = Encoding.UTF8.GetBytes(ExperimentRequestV2Canonicalizer.Canonicalize(request).Content);
        var expected = File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", $"2026-10-30-synthetic-request-v2-{name}.json"));
        Assert.Equal(expected, bytes);
        Assert.Equal(GoldenHashes[name], RequestFingerprint.FromRequest(request).Sha256);
        Assert.Equal(GoldenHashes[name], Convert.ToHexStringLower(SHA256.HashData(bytes)));
        using var json = JsonDocument.Parse(bytes);
        Assert.Equal(new[] { "canonicalizationVersion", "datasetId", "strategyVersion", "windowStartUtc", "windowEndUtc", "quoteTokenId", "tokenIds", "poolIds", "routeShapes", "priceInputPoolIds", "minTradeSizeQuoteAtoms", "maxTradeSizeQuoteAtoms" }, json.RootElement.EnumerateObject().Select(p => p.Name));
        Assert.Equal(2, json.RootElement.GetProperty("canonicalizationVersion").GetInt32());
    }
    static readonly Dictionary<string, string> GoldenHashes = new()
    {
        ["r1"] = "PENDING", ["r2"] = "PENDING", ["r3"] = "PENDING"
    };

    [Fact] public void CanonicalSortingCultureOffsetsAndInvalidInput()
    {
        var baseline = ExperimentRequestV2Canonicalizer.Canonicalize(R2());
        var previous = CultureInfo.CurrentCulture;
        try
        {
            CultureInfo.CurrentCulture = new("tr-TR");
            var reordered = Request(tokens: [CaptureReplayCatalog.Usdt, CaptureReplayCatalog.Usdc, CaptureReplayCatalog.Sol],
                pools: AllPools().Reverse().ToArray(), shapes: ["triangle", "two_leg"],
                start: Start.ToOffset(TimeSpan.FromHours(2)), end: End.ToOffset(TimeSpan.FromHours(-3)));
            Assert.Equal(baseline.Content, ExperimentRequestV2Canonicalizer.Canonicalize(reordered).Content);
            Assert.Equal(RequestFingerprint.FromRequest(R2()), RequestFingerprint.FromRequest(reordered));
        }
        finally { CultureInfo.CurrentCulture = previous; }
        Assert.NotEqual(baseline.Content, ExperimentRequestV2Canonicalizer.Canonicalize(Request(max: 999_999_999, shapes: ["two_leg", "triangle"])).Content);
        Assert.Throws<ArgumentNullException>(() => ExperimentRequestV2Canonicalizer.Canonicalize(null!));
        Assert.Throws<ArgumentException>(() => ExperimentRequestV2Canonicalizer.Canonicalize(Request(min: 0)));
    }

    [Fact] public void V1FixtureFingerprintsStayUnchanged()
    {
        foreach (var pair in new[] { 'a', 'b', 'c' })
        {
            var requestBytes = File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", $"2026-10-30-replay-{pair}-request.json"));
            using var requestJson = JsonDocument.Parse(requestBytes); var root = requestJson.RootElement;
            var request = new ExperimentRequest(root.GetProperty("datasetId").GetString(), root.GetProperty("strategyVersion").GetString(),
                root.GetProperty("windowStartUtc").GetDateTimeOffset(), root.GetProperty("windowEndUtc").GetDateTimeOffset(),
                root.GetProperty("tokenPair").EnumerateArray().Select(x => x.GetString()).ToArray(),
                root.GetProperty("poolIds").EnumerateArray().Select(x => x.GetString()).ToArray(),
                root.GetProperty("minTradeSizeQuoteAtoms").GetInt64(), root.GetProperty("maxTradeSizeQuoteAtoms").GetInt64());
            Assert.Equal(requestBytes, Encoding.UTF8.GetBytes(ExperimentRequestCanonicalizer.Canonicalize(request).Content));
            using var resultJson = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", $"2026-10-30-replay-{pair}-result.json")));
            Assert.Equal(resultJson.RootElement.GetProperty("requestSha256").GetString(), RequestFingerprint.FromRequest(request).Sha256);
        }
    }

    [Fact] public void FingerprintsAndResultLinkageAreVersionSeparated()
    {
        var pools = CaptureReplayCatalog.Pools('a').Take(2).ToArray();
        var v1 = new ExperimentRequest(CaptureReplayCatalog.DatasetId, CaptureReplayCatalog.StrategyVersion, Start, End,
            [CaptureReplayCatalog.Sol, CaptureReplayCatalog.Usdc], pools, 1, 1_000_000_000);
        var v2 = Request(tokens: [CaptureReplayCatalog.Sol, CaptureReplayCatalog.Usdc], pools: pools,
            shapes: ["two_leg"], price: []);
        var f1 = RequestFingerprint.FromRequest(v1); var f2 = RequestFingerprint.FromRequest(v2);
        Assert.NotEqual(f1, f2); Assert.Equal(1, f1.CanonicalizationVersion); Assert.Equal(2, f2.CanonicalizationVersion);
        ExperimentResult Result(RequestFingerprint f) => new("synthetic-contract-result", f.Sha256, f.CanonicalizationVersion,
            CaptureReplayCatalog.DatasetId, "capture-v1", CaptureReplayCatalog.StrategyVersion, "contract-test",
            "cost-test", Start, Start, End, EvidenceKind.Synthetic, ExperimentResultStatus.Completed, null,
            Enum.GetValues<PolicyKind>().Select(k => (PolicyResult?)new PolicyResult(k, 0, 0, 0, [], null, null, null, null)).ToArray());
        var r1 = Result(f1); var r2 = Result(f2); var catalog = CaptureReplayCatalog.Create();
        Assert.Empty(ExperimentResultValidator.Validate(r1, catalog, v1));
        Assert.Empty(ExperimentResultValidator.Validate(r2, catalog, v2));
        Assert.Contains(new ValidationIssue("request", "request_mismatch"), ExperimentResultValidator.Validate(r1, catalog, v2));
        Assert.Contains(new ValidationIssue("requestCanonicalizationVersion", "unsupported_canonicalization_version"), ExperimentResultValidator.Validate(r2, catalog, v1));
        Assert.Contains(new ValidationIssue("request", "invalid_request"), ExperimentResultValidator.Validate(r2, catalog, (ExperimentRequestV2?)null));
    }
}
