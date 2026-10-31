using MEVana.Research.Api;

var configuration = new ConfigurationBuilder()
    .AddEnvironmentVariables("MEVANA_RESEARCH_")
    .AddCommandLine(args)
    .Build();
var app = await ResearchApiApp.StartAsync(new(
    configuration["ArtifactDirectory"] ?? Path.Combine(AppContext.BaseDirectory, "research-artifacts"),
    configuration["Url"] ?? ResearchApiApp.DefaultUrl));
await app.WaitForShutdownAsync();
