using System.Text.Json;
using MEVana.Research.Core;

namespace MEVana.Research.Persistence;

/// <summary>Strict document parsers. A request document is accepted only by its own contract version.</summary>
public static class ResearchArtifactParser
{
    public static ExperimentRequest ParseRequestV1(byte[] bytes, string artifact = "request")
    {
        var json = new ArtifactJson(artifact);
        using var document = json.Parse(bytes);
        var root = document.RootElement;
        RequireVersion(json, root, 1);
        json.Exact(root, "canonicalizationVersion", "datasetId", "strategyVersion", "windowStartUtc", "windowEndUtc",
            "tokenPair", "poolIds", "minTradeSizeQuoteAtoms", "maxTradeSizeQuoteAtoms");
        return new(json.Text(root, "datasetId"), json.Text(root, "strategyVersion"),
            json.Time(root, "windowStartUtc"), json.Time(root, "windowEndUtc"),
            json.Texts(root, "tokenPair"), json.Texts(root, "poolIds"),
            json.Number(root, "minTradeSizeQuoteAtoms"), json.Number(root, "maxTradeSizeQuoteAtoms"));
    }

    public static ExperimentRequestV2 ParseRequestV2(byte[] bytes, string artifact = "request")
    {
        var json = new ArtifactJson(artifact);
        using var document = json.Parse(bytes);
        var root = document.RootElement;
        RequireVersion(json, root, 2);
        json.Exact(root, "canonicalizationVersion", "datasetId", "strategyVersion", "windowStartUtc", "windowEndUtc",
            "quoteTokenId", "tokenIds", "poolIds", "routeShapes", "priceInputPoolIds",
            "minTradeSizeQuoteAtoms", "maxTradeSizeQuoteAtoms");
        return new(json.Text(root, "datasetId"), json.Text(root, "strategyVersion"),
            json.Time(root, "windowStartUtc"), json.Time(root, "windowEndUtc"), json.Text(root, "quoteTokenId"),
            json.Texts(root, "tokenIds"), json.Texts(root, "poolIds"), json.Texts(root, "routeShapes"),
            json.Texts(root, "priceInputPoolIds"),
            json.Number(root, "minTradeSizeQuoteAtoms"), json.Number(root, "maxTradeSizeQuoteAtoms"));
    }

    public static ExperimentResult ParseResult(byte[] bytes, string artifact = "result")
    {
        var json = new ArtifactJson(artifact);
        using var document = json.Parse(bytes);
        var root = document.RootElement;
        RequireVersion(json, root, 1);
        json.Exact(root, "canonicalizationVersion", "resultId", "requestSha256", "requestCanonicalizationVersion",
            "datasetId", "datasetMetadataVersion", "strategyVersion", "engineVersion", "costModelVersion",
            "createdAtUtc", "windowStartUtc", "windowEndUtc", "evidenceKind", "status", "errorCode", "policies");
        var policies = json.Items(root, "policies").Select(policy => (PolicyResult?)Policy(json, policy)).ToArray();
        return new(json.Text(root, "resultId"), json.Text(root, "requestSha256"),
            (int)json.Number(root, "requestCanonicalizationVersion"), json.Text(root, "datasetId"),
            json.Text(root, "datasetMetadataVersion"), json.Text(root, "strategyVersion"),
            json.Text(root, "engineVersion"), json.Text(root, "costModelVersion"), json.Time(root, "createdAtUtc"),
            json.Time(root, "windowStartUtc"), json.Time(root, "windowEndUtc"),
            json.Named<EvidenceKind>(root, "evidenceKind"), json.Named<ExperimentResultStatus>(root, "status"),
            json.OptionalText(root, "errorCode"), policies);
    }

    public static ResearchCatalog ParseCatalog(byte[] bytes, string artifact = "catalog")
    {
        var json = new ArtifactJson(artifact);
        using var document = json.Parse(bytes);
        var root = document.RootElement;
        json.Exact(root, "datasets");
        try { return new(json.Items(root, "datasets").Select(dataset => Dataset(json, dataset)).ToArray()); }
        catch (ArgumentException) { throw json.Fail("invalid_value"); }
    }

    static void RequireVersion(ArtifactJson json, JsonElement root, int version)
    {
        if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty("canonicalizationVersion", out var value)
            || value.ValueKind != JsonValueKind.Number || value.GetInt64() != version)
            throw json.Fail("contract_version_mismatch");
    }

    static PolicyResult Policy(ArtifactJson json, JsonElement policy)
    {
        json.Exact(policy, "kind", "candidatesEvaluated", "opportunitiesDetected", "selectedOrExecuted",
            "rejectedCandidates", "missedCandidates", "estimatedNetProfitQuoteAtoms",
            "simulatedNetProfitQuoteAtoms", "realizedNetProfitQuoteAtoms");
        var rejected = json.Items(policy, "rejectedCandidates").Select(rejection =>
        {
            json.Exact(rejection, "reasonCode", "count");
            return (RejectedCandidateCount?)new RejectedCandidateCount(json.Text(rejection, "reasonCode"), json.Number(rejection, "count"));
        }).ToArray();
        return new(json.Named<PolicyKind>(policy, "kind"), json.Number(policy, "candidatesEvaluated"),
            json.Number(policy, "opportunitiesDetected"), json.Number(policy, "selectedOrExecuted"), rejected,
            json.OptionalNumber(policy, "missedCandidates"), json.OptionalNumber(policy, "estimatedNetProfitQuoteAtoms"),
            json.OptionalNumber(policy, "simulatedNetProfitQuoteAtoms"), json.OptionalNumber(policy, "realizedNetProfitQuoteAtoms"));
    }

    static DatasetMetadata Dataset(ArtifactJson json, JsonElement dataset)
    {
        json.Exact(dataset, "id", "metadataVersion", "provenance", "verifiedAvailable", "suitability",
            "coverageStartUtc", "coverageEndUtc", "supportedStrategyVersions", "tokens", "pools",
            "maxResearchTradeSizeQuoteAtoms");
        var tokens = json.Items(dataset, "tokens").Select(token =>
        {
            json.Exact(token, "id", "decimals", "maxResearchTradeSizeQuoteAtoms");
            return new TokenMetadata(json.Text(token, "id"), (int)json.Number(token, "decimals"),
                json.Number(token, "maxResearchTradeSizeQuoteAtoms"));
        }).ToArray();
        var pools = json.Items(dataset, "pools").Select(pool =>
        {
            json.Exact(pool, "id", "tokenIds");
            return new PoolMetadata(json.Text(pool, "id"), json.Texts(pool, "tokenIds"));
        }).ToArray();
        return new(json.Text(dataset, "id"), json.Text(dataset, "metadataVersion"), json.Text(dataset, "provenance"),
            json.Flag(dataset, "verifiedAvailable"), json.Text(dataset, "suitability"), json.Time(dataset, "coverageStartUtc"),
            json.Time(dataset, "coverageEndUtc"), json.Texts(dataset, "supportedStrategyVersions"), tokens, pools,
            json.Number(dataset, "maxResearchTradeSizeQuoteAtoms"));
    }
}
