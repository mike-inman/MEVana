using System.Net;
using MEVana.Research.Persistence;
using Microsoft.AspNetCore.Hosting.Server;
using Microsoft.AspNetCore.Hosting.Server.Features;

namespace MEVana.Research.Api;

public sealed record ResearchApiOptions(string ArtifactDirectory, string Url = ResearchApiApp.DefaultUrl,
    Action<ILoggingBuilder>? ConfigureLogging = null);

/// <summary>
/// Read-only, loopback-only host. There is no authentication, so the loopback bind
/// is this API's only access control.
/// </summary>
public static class ResearchApiApp
{
    public const string DefaultUrl = "http://127.0.0.1:8098";
    static readonly byte[] NotFound = """{"code":"not_found"}"""u8.ToArray();
    static readonly byte[] MethodNotAllowed = """{"code":"method_not_allowed"}"""u8.ToArray();
    static readonly byte[] Health = """{"status":"ok"}"""u8.ToArray();
    static readonly string[] ReadMethods = ["GET", "HEAD"];

    /// <summary>Validates artifacts before binding, then starts. Throws if anything is not safe to serve.</summary>
    public static async Task<WebApplication> StartAsync(ResearchApiOptions options)
    {
        var endpoint = LoopbackEndpoint(options.Url);
        var responses = new ResearchResponses(ResearchArtifactStore.Load(options.ArtifactDirectory));

        var builder = WebApplication.CreateSlimBuilder(new WebApplicationOptions { ContentRootPath = AppContext.BaseDirectory });
        builder.WebHost.ConfigureKestrel(kestrel => kestrel.Listen(endpoint));
        builder.Logging.ClearProviders();
        builder.Logging.AddFilter("Microsoft", LogLevel.Warning);
        if (options.ConfigureLogging is null) builder.Logging.AddSimpleConsole();
        else options.ConfigureLogging(builder.Logging);

        var app = builder.Build();
        var log = app.Services.GetRequiredService<ILoggerFactory>().CreateLogger("MEVana.Research.Api.Requests");
        app.UseRouting();
        app.Use(async (context, next) =>
        {
            await next();
            var template = (context.GetEndpoint() as RouteEndpoint)?.RoutePattern.RawText ?? "(unmatched)";
            var method = ReadMethods.Contains(context.Request.Method) ? context.Request.Method : "OTHER";
            log.LogInformation("{Method} {Template} {Status}", method, template, context.Response.StatusCode);
        });
        app.Use((context, next) =>
        {
            if (ReadMethods.Contains(context.Request.Method)) return next(context);
            context.Response.Headers.Allow = "GET, HEAD";
            return Json(context, MethodNotAllowed, StatusCodes.Status405MethodNotAllowed);
        });

        Map(app, "/health", _ => Health);
        Map(app, "/research/v1/datasets", _ => responses.DatasetList);
        Map(app, "/research/v1/datasets/{datasetId}", context => Find(responses.Datasets, context, "datasetId"));
        Map(app, "/research/v1/experiments", _ => responses.ExperimentList);
        Map(app, "/research/v1/experiments/{resultId}", context => Find(responses.Experiments, context, "resultId"));
        app.MapFallback("{*path}", context => Json(context, NotFound, StatusCodes.Status404NotFound));

        await app.StartAsync();
        var addresses = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>()!.Addresses;
        if (addresses.Count == 0 || addresses.Any(address => !IPAddress.IsLoopback(IPAddress.Parse(new Uri(address).Host.Trim('[', ']')))))
        {
            await app.StopAsync();
            throw new InvalidOperationException("Research API may bind only to loopback.");
        }
        return app;
    }

    /// <summary>Accepts only a plain http URL with a literal loopback address.</summary>
    public static IPEndPoint LoopbackEndpoint(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri) || uri.Scheme != Uri.UriSchemeHttp
            || uri.PathAndQuery != "/" || uri.UserInfo.Length != 0
            || !IPAddress.TryParse(uri.Host.Trim('[', ']'), out var address) || !IPAddress.IsLoopback(address))
            throw new ArgumentException("Research API may bind only to a literal loopback http address.", nameof(url));
        return new(address, uri.Port);
    }

    static void Map(WebApplication app, string pattern, Func<HttpContext, byte[]?> body) =>
        app.MapMethods(pattern, ReadMethods, context =>
            body(context) is { } bytes
                ? Json(context, bytes, StatusCodes.Status200OK)
                : Json(context, NotFound, StatusCodes.Status404NotFound));

    static byte[]? Find(IReadOnlyDictionary<string, byte[]> values, HttpContext context, string key) =>
        values.GetValueOrDefault((string)context.Request.RouteValues[key]!);

    static Task Json(HttpContext context, byte[] body, int status)
    {
        context.Response.StatusCode = status;
        context.Response.ContentType = "application/json";
        context.Response.ContentLength = body.Length;
        return HttpMethods.IsHead(context.Request.Method) ? Task.CompletedTask : context.Response.Body.WriteAsync(body).AsTask();
    }
}
