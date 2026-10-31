using System.Net;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MEVana.Research.Api;
using MEVana.Research.Persistence;
using Microsoft.AspNetCore.Builder;
using Microsoft.Extensions.Logging;
using Xunit;

namespace MEVana.Research.Tests;

// 2026-10-30 CDT. Starts the real host on an ephemeral loopback port.
public sealed class ResearchApiFixture : IAsyncLifetime
{
    internal static readonly string Seed = Path.Combine(AppContext.BaseDirectory, "research-artifacts");
    internal readonly List<string> LogLines = [];
    WebApplication _app = null!;
    internal HttpClient Client { get; private set; } = null!;

    public async Task InitializeAsync()
    {
        _app = await ResearchApiApp.StartAsync(new(Seed, "http://127.0.0.1:0",
            logging => logging.AddProvider(new CapturingProvider(LogLines))));
        Client = new() { BaseAddress = new(_app.Urls.Single()) };
    }

    public async Task DisposeAsync()
    {
        Client.Dispose();
        await _app.StopAsync();
        await _app.DisposeAsync();
    }

    sealed class CapturingProvider(List<string> lines) : ILoggerProvider, ILogger
    {
        public ILogger CreateLogger(string categoryName) => this;
        public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;
        public bool IsEnabled(LogLevel logLevel) => true;
        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
        {
            lock (lines) lines.Add(formatter(state, exception));
        }
        public void Dispose() { }
    }
}

public class ResearchApiEndpointTests(ResearchApiFixture api) : IClassFixture<ResearchApiFixture>
{
    const string Dataset = "orca-wp-snapshot-20261030";
    const string V2 = "replay-v2-463353643-usdc-confirmed-l1-tip10000";
    const string PairB = "replay-v2-463353643-b-confirmed-l1-tip10000";
    const string Usdc = CaptureReplayCatalog.Usdc;
    const string Usdt = CaptureReplayCatalog.Usdt;

    async Task<JsonElement> Get(string path)
    {
        using var response = await api.Client.GetAsync(path);
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Equal("application/json", response.Content.Headers.ContentType?.MediaType);
        using var document = JsonDocument.Parse(await response.Content.ReadAsByteArrayAsync());
        return document.RootElement.Clone();
    }

    async Task<JsonElement[]> Experiments() =>
        (await Get("/research/v1/experiments")).GetProperty("experiments").EnumerateArray().ToArray();

    [Fact] public async Task HealthCarriesNoData() =>
        Assert.Equal("""{"status":"ok"}""", await api.Client.GetStringAsync("/health"));

    [Fact] public async Task DatasetsListAndDetail()
    {
        var list = (await Get("/research/v1/datasets")).GetProperty("datasets");
        Assert.Equal(1, list.GetArrayLength());
        var detail = await Get($"/research/v1/datasets/{Dataset}");
        Assert.Equal(list[0].GetRawText(), detail.GetRawText());
        Assert.Equal(13, detail.GetProperty("pools").GetArrayLength());
        Assert.Equal("Simulated research only", detail.GetProperty("suitability").GetString());
    }

    [Fact] public async Task ExperimentsAreOrderedAndLabeled()
    {
        var experiments = await Experiments();
        var ids = experiments.Select(e => e.GetProperty("resultId").GetString()!).ToArray();
        Assert.Equal(4, ids.Length);
        Assert.Equal(ids.Order(StringComparer.Ordinal), ids);
        Assert.All(experiments, e =>
        {
            Assert.Equal("Simulated", e.GetProperty("evidenceKind").GetString());
            Assert.Equal(3, e.GetProperty("coverage").GetArrayLength());
            Assert.Equal(6, e.GetProperty("quoteTokenDecimals").GetInt32());
        });
    }

    [Theory]
    [InlineData("replay-v2-463353643-a-confirmed-l1-tip10000", Usdc, 1, 6, 67050, 180)]
    [InlineData("replay-v2-463353643-b-confirmed-l1-tip10000", Usdt, 1, 5, 44700, 24639)]
    [InlineData("replay-v2-463353643-c-confirmed-l1-tip10000", Usdt, 1, 2, 4470, 12)]
    [InlineData("replay-v2-463353643-usdc-confirmed-l1-tip10000", Usdc, 2, 13, 339720, 81156)]
    public async Task QuoteAssetScopeAndCoveragePerExperiment(string id, string quote, int contract, int pools, long evaluated, long unevaluable)
    {
        var e = await Get($"/research/v1/experiments/{id}");
        Assert.Equal(quote, e.GetProperty("quoteTokenId").GetString());
        Assert.Equal(contract, e.GetProperty("requestCanonicalizationVersion").GetInt32());
        Assert.Equal(pools, e.GetProperty("scope").GetProperty("poolCount").GetInt32());
        Assert.All(e.GetProperty("coverage").EnumerateArray(), c =>
        {
            Assert.Equal(evaluated, c.GetProperty("candidatesEvaluated").GetInt64());
            Assert.Equal(unevaluable, c.GetProperty("unevaluableCandidates").GetInt64());
            Assert.Equal(evaluated - unevaluable, c.GetProperty("evaluableCandidates").GetInt64());
        });
    }

    [Fact] public async Task NotMeasuredStaysNull()
    {
        foreach (var experiment in await Experiments())
            Assert.All(experiment.GetProperty("policies").EnumerateArray(), policy =>
            {
                Assert.Equal(JsonValueKind.Null, policy.GetProperty("missedCandidates").ValueKind);
                Assert.Equal(JsonValueKind.Null, policy.GetProperty("realizedNetProfitQuoteAtoms").ValueKind);
            });
    }

    [Fact] public async Task V2DetailCarriesRoutesShapesAndLabeledProbes()
    {
        var e = await Get($"/research/v1/experiments/{V2}");
        var routes = e.GetProperty("routeCounts");
        Assert.Equal(120, routes.GetProperty("triangle").GetInt32());
        Assert.Equal(32, routes.GetProperty("two_leg").GetInt32());
        Assert.Equal(152, routes.GetProperty("total").GetInt32());
        Assert.Equal(268200, e.GetProperty("shapes").GetProperty("triangle").GetProperty("policies")[0].GetProperty("candidatesEvaluated").GetInt64());
        var probes = e.GetProperty("poolDirections");
        Assert.True(probes.GetProperty("diagnosticProbe").GetBoolean());
        Assert.Equal(26, probes.GetProperty("rows").GetArrayLength());
        Assert.NotEmpty(e.GetProperty("limitations").EnumerateArray());
        Assert.EndsWith("handoff-14-multi-route-replay-closeout.md", e.GetProperty("sourceCloseout").GetString());
    }

    [Fact] public async Task V1DetailHasNoV2OnlyData()
    {
        var e = await Get($"/research/v1/experiments/{PairB}");
        foreach (var name in new[] { "routeCounts", "shapes", "poolDirections" })
            Assert.Equal(JsonValueKind.Null, e.GetProperty(name).ValueKind);
    }

    [Theory] [InlineData("replay-v2-463353643-a-confirmed-l1-tip10000")] [InlineData(V2)]
    public async Task ClientCanRecomputeTheRequestHash(string id)
    {
        var e = await Get($"/research/v1/experiments/{id}");
        var request = e.GetProperty("request").GetRawText();
        Assert.Equal(e.GetProperty("requestSha256").GetString(), Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(request))));
    }

    [Theory]
    [InlineData("/nope")] [InlineData("/x.txt")] [InlineData("/research/v1")] [InlineData("/research/v1/datasets/missing")]
    [InlineData("/research/v1/experiments/missing")] [InlineData("/research/v1/experiments/%2e%2e%2fsecret-marker")]
    [InlineData("/research/v1/experiments/<script>secret-marker")]
    public async Task UnknownOrMalformedReturnsPlainNotFound(string path)
    {
        using var response = await api.Client.GetAsync(path);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
        Assert.Equal("""{"code":"not_found"}""", await response.Content.ReadAsStringAsync());
    }

    [Theory]
    [InlineData("POST", "/research/v1/experiments")] [InlineData("PUT", "/research/v1/datasets")]
    [InlineData("PATCH", "/research/v1/experiments/" + V2)] [InlineData("DELETE", "/health")]
    [InlineData("POST", "/nope")]
    public async Task OtherMethodsReturn405(string method, string path)
    {
        using var response = await api.Client.SendAsync(new(new HttpMethod(method), path));
        Assert.Equal(HttpStatusCode.MethodNotAllowed, response.StatusCode);
        Assert.Equal(new[] { "GET", "HEAD" }, response.Content.Headers.Allow.Order());
    }

    [Fact] public async Task HeadReturnsHeadersOnly()
    {
        using var response = await api.Client.SendAsync(new(HttpMethod.Head, "/research/v1/experiments"));
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.Empty(await response.Content.ReadAsByteArrayAsync());
    }

    [Fact] public async Task ResponsesMakeNoVerdictAndNoCrossExperimentTotal()
    {
        var body = await api.Client.GetStringAsync("/research/v1/experiments");
        foreach (var word in new[] { "profitable", "successful", "recommended", "totalProfit" })
            Assert.DoesNotContain(word, body, StringComparison.OrdinalIgnoreCase);
        Assert.Equal(new[] { "experiments" }, (await Get("/research/v1/experiments")).EnumerateObject().Select(p => p.Name));
    }

    [Theory]
    [InlineData("/research/v1/experiments", "2026-10-30-research-api-experiments.json")]
    [InlineData("/research/v1/experiments/" + V2, "2026-10-30-research-api-v2-experiment.json")]
    public async Task GoldenResponses(string path, string fixture) => Assert.Equal(
        File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", fixture)), await api.Client.GetByteArrayAsync(path));

    [Fact] public async Task TwoStartsReturnIdenticalBytes()
    {
        var second = await ResearchApiApp.StartAsync(new(ResearchApiFixture.Seed, "http://127.0.0.1:0", _ => { }));
        try
        {
            using var client = new HttpClient { BaseAddress = new(second.Urls.Single()) };
            foreach (var path in new[] { "/research/v1/datasets", "/research/v1/experiments", "/research/v1/experiments/" + V2 })
                Assert.Equal(await api.Client.GetByteArrayAsync(path), await client.GetByteArrayAsync(path));
        }
        finally { await second.StopAsync(); await second.DisposeAsync(); }
    }

    [Fact] public async Task RequestLogKeepsOnlyMethodTemplateAndStatus()
    {
        using var response = await api.Client.GetAsync("/research/v1/experiments/log-secret-marker");
        await Task.Delay(50);
        string[] lines;
        lock (api.LogLines) lines = api.LogLines.ToArray();
        Assert.Contains("GET /research/v1/experiments/{resultId} 404", lines);
        Assert.DoesNotContain(lines, line => line.Contains("log-secret-marker", StringComparison.Ordinal));
    }
}

public class ResearchApiBindGuardTests
{
    [Theory] [InlineData("http://127.0.0.1:8098")] [InlineData("http://127.0.0.2:8098")] [InlineData("http://[::1]:8098")]
    public void LoopbackIsAccepted(string url) => Assert.True(IPAddress.IsLoopback(ResearchApiApp.LoopbackEndpoint(url).Address));

    [Theory]
    [InlineData("http://0.0.0.0:8098")] [InlineData("http://[::]:8098")] [InlineData("http://*:8098")] [InlineData("http://+:8098")]
    [InlineData("http://8.8.8.8:8098")] [InlineData("http://192.168.1.10:8098")] [InlineData("http://localhost:8098")]
    [InlineData("https://127.0.0.1:8098")] [InlineData("http://127.0.0.1:8098/base")] [InlineData("not a url")]
    public async Task NonLoopbackFailsStartup(string url)
    {
        await Assert.ThrowsAsync<ArgumentException>(() => ResearchApiApp.StartAsync(new(ResearchApiFixture.Seed, url)));
    }
}
