using System.Security.Cryptography;
using System.Text;

namespace MEVana.Research.Core;

public sealed record RequestFingerprint(int CanonicalizationVersion, string Sha256)
{
    public static RequestFingerprint FromRequest(ExperimentRequest request)
    {
        var canonical = ExperimentRequestCanonicalizer.Canonicalize(request);
        return new(canonical.Version, Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(canonical.Content))));
    }
}

public static class ExperimentResultValidator
{
    /// <summary>Checks declared provenance and catalog compatibility; does not verify original request content.</summary>
    public static IReadOnlyList<ValidationIssue> Validate(ExperimentResult? result, ResearchCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        var syntax = ExperimentResultSyntaxValidator.Validate(result);
        if (syntax.Count != 0) return syntax;
        var r = result!;
        var errors = new List<ValidationIssue>();
        var dataset = catalog.FindDataset(r.DatasetId!);
        if (dataset is null) { errors.Add(new("datasetId", "unknown_dataset")); return errors.AsReadOnly(); }
        if (!string.Equals(dataset.MetadataVersion, r.DatasetMetadataVersion, StringComparison.Ordinal))
            errors.Add(new("datasetMetadataVersion", "metadata_version_mismatch"));
        if (!dataset.SupportedStrategyVersions.Contains(r.StrategyVersion!, StringComparer.Ordinal))
            errors.Add(new("strategyVersion", "unsupported_strategy"));
        if (r.WindowStartUtc < dataset.CoverageStartUtc || r.WindowEndUtc > dataset.CoverageEndUtc)
            errors.Add(new("window", "invalid_window"));
        return errors.AsReadOnly();
    }

    /// <summary>Also verifies the SHA-256 fingerprint and common scope against the original raw request.</summary>
    public static IReadOnlyList<ValidationIssue> Validate(ExperimentResult? result, ResearchCatalog catalog, ExperimentRequest? request)
    {
        var errors = Validate(result, catalog).ToList();
        if (ExperimentResultSyntaxValidator.Validate(result).Count != 0) return errors.AsReadOnly();
        if (ExperimentRequestSyntaxValidator.Validate(request).Count != 0)
        {
            errors.Add(new("request", "invalid_request"));
            return errors.AsReadOnly();
        }
        var fingerprint = RequestFingerprint.FromRequest(request!);
        var r = result!;
        if (r.RequestSha256 != fingerprint.Sha256 || r.RequestCanonicalizationVersion != fingerprint.CanonicalizationVersion
            || r.DatasetId != request!.DatasetId || r.StrategyVersion != request.StrategyVersion
            || r.WindowStartUtc != request.WindowStartUtc || r.WindowEndUtc != request.WindowEndUtc)
            errors.Add(new("request", "request_mismatch"));
        return errors.AsReadOnly();
    }
}
