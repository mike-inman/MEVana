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

    public static RequestFingerprint FromRequest(ExperimentRequestV2 request, bool versionTwo = true)
    {
        var canonical = ExperimentRequestV2Canonicalizer.Canonicalize(request);
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

    /// <summary>Checks v2 linkage while retaining the unchanged v1-only result syntax path.</summary>
    public static IReadOnlyList<ValidationIssue> Validate(ExperimentResult? result, ResearchCatalog catalog, ExperimentRequestV2? request, bool versionTwo = true)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        if (result is null) return Validate(result, catalog);
        // Result syntax accepts version 1 only. Normalize that single field in a copy
        // to reuse every other established syntax and catalog check without widening v1.
        var normalized = new ExperimentResult(result.ResultId, result.RequestSha256, 1,
            result.DatasetId, result.DatasetMetadataVersion, result.StrategyVersion,
            result.EngineVersion, result.CostModelVersion, result.CreatedAtUtc,
            result.WindowStartUtc, result.WindowEndUtc, result.EvidenceKind, result.Status,
            result.ErrorCode, result.Policies);
        var errors = Validate(normalized, catalog).ToList();
        if (result.RequestCanonicalizationVersion is null)
            errors.Add(new("requestCanonicalizationVersion", "required"));
        if (ExperimentResultSyntaxValidator.Validate(normalized).Count != 0
            || result.RequestCanonicalizationVersion is null) return errors.AsReadOnly();
        if (ExperimentRequestV2SyntaxValidator.Validate(request).Count != 0)
        {
            errors.Add(new("request", "invalid_request"));
            return errors.AsReadOnly();
        }
        var fingerprint = RequestFingerprint.FromRequest(request!);
        if (result.RequestCanonicalizationVersion != 2 || result.RequestSha256 != fingerprint.Sha256
            || result.DatasetId != request!.DatasetId || result.StrategyVersion != request.StrategyVersion
            || result.WindowStartUtc != request.WindowStartUtc || result.WindowEndUtc != request.WindowEndUtc)
            errors.Add(new("request", "request_mismatch"));
        return errors.AsReadOnly();
    }
}
