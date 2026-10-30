namespace MEVana.Research.Core;

public static class ExperimentResultV2SyntaxValidator
{
    public static IReadOnlyList<ValidationIssue> Validate(ExperimentResult? result)
    {
        if (result is null) return ExperimentResultSyntaxValidator.Validate(null);
        var normalized = new ExperimentResult(result.ResultId, result.RequestSha256, 1,
            result.DatasetId, result.DatasetMetadataVersion, result.StrategyVersion,
            result.EngineVersion, result.CostModelVersion, result.CreatedAtUtc,
            result.WindowStartUtc, result.WindowEndUtc, result.EvidenceKind, result.Status,
            result.ErrorCode, result.Policies);
        var issues = ExperimentResultSyntaxValidator.Validate(normalized).ToList();
        if (result.RequestCanonicalizationVersion != 2)
            issues.Add(new("requestCanonicalizationVersion", result.RequestCanonicalizationVersion is null
                ? "required" : "unsupported_canonicalization_version"));
        return issues.AsReadOnly();
    }
}
