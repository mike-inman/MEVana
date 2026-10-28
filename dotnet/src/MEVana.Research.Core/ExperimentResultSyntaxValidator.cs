namespace MEVana.Research.Core;

public static class ExperimentResultSyntaxValidator
{
    static bool Id(string? value) => value is { Length: > 0 and <= 64 }
        && value.All(c => char.IsAsciiLetterOrDigit(c) || c is '-' or '_');

    public static IReadOnlyList<ValidationIssue> Validate(ExperimentResult? result)
    {
        var errors = new List<ValidationIssue>();
        void Add(string field, string code) => errors.Add(new(field, code));
        void Identifier(string field, string? value)
        {
            if (value is null) Add(field, "required");
            else if (!Id(value)) Add(field, "invalid_identifier");
        }
        void Utc(string field, DateTimeOffset? value)
        {
            if (value is null) Add(field, "required");
            else if (value.Value.Offset != TimeSpan.Zero) Add(field, "invalid_utc");
        }
        void EnumValue<T>(string field, T? value) where T : struct, Enum
        {
            if (value is null) Add(field, "required");
            else if (!Enum.IsDefined(value.Value)) Add(field, "invalid_enum");
        }
        void Count(string field, long? value, bool required = true)
        {
            if (value is null && required) Add(field, "required");
            else if (value < 0) Add(field, "invalid_count");
        }

        if (result is null) { Add("result", "required"); return errors.AsReadOnly(); }
        Identifier("resultId", result.ResultId);
        Identifier("datasetId", result.DatasetId);
        Identifier("datasetMetadataVersion", result.DatasetMetadataVersion);
        Identifier("strategyVersion", result.StrategyVersion);
        Identifier("engineVersion", result.EngineVersion);
        Identifier("costModelVersion", result.CostModelVersion);
        if (result.RequestSha256 is null) Add("requestSha256", "required");
        else if (result.RequestSha256.Length != 64 || !result.RequestSha256.All(c => c is >= '0' and <= '9' or >= 'a' and <= 'f'))
            Add("requestSha256", "invalid_hash");
        if (result.RequestCanonicalizationVersion is null) Add("requestCanonicalizationVersion", "required");
        else if (result.RequestCanonicalizationVersion != 1) Add("requestCanonicalizationVersion", "unsupported_canonicalization_version");
        Utc("createdAtUtc", result.CreatedAtUtc);
        Utc("windowStartUtc", result.WindowStartUtc);
        Utc("windowEndUtc", result.WindowEndUtc);
        if (result.WindowStartUtc is { } start && result.WindowEndUtc is { } end
            && (start >= end || end - start > TimeSpan.FromHours(24))) Add("window", "invalid_window");
        EnumValue("evidenceKind", result.EvidenceKind);
        EnumValue("status", result.Status);

        if (result.Status is ExperimentResultStatus.Failed or ExperimentResultStatus.Unavailable)
        {
            Identifier("errorCode", result.ErrorCode);
            if (result.Policies is { Count: > 0 }) Add("policies", "metrics_not_allowed");
        }
        else if (result.Status == ExperimentResultStatus.Completed)
        {
            if (result.ErrorCode is not null) Add("errorCode", "error_not_allowed");
            if (result.Policies is null) Add("policies", "required");
            else
            {
                if (result.Policies.Count != 3 || result.Policies.Any(p => p is null)
                    || result.Policies.Select(p => p?.Kind).Distinct().Count() != result.Policies.Count)
                    Add("policies", "invalid_policy_set");
                foreach (var kind in Enum.GetValues<PolicyKind>())
                    if (!result.Policies.Any(p => p?.Kind == kind)) Add("policies", "missing_policy");
            }
        }

        if (result.Policies is not null)
        for (int i = 0; i < result.Policies.Count; i++)
        {
            var p = result.Policies[i];
            var field = $"policies[{i}]";
            if (p is null) { Add(field, "required"); continue; }
            EnumValue(field + ".kind", p.Kind);
            Count(field + ".candidatesEvaluated", p.CandidatesEvaluated);
            Count(field + ".opportunitiesDetected", p.OpportunitiesDetected);
            Count(field + ".selectedOrExecuted", p.SelectedOrExecuted);
            Count(field + ".missedCandidates", p.MissedCandidates, false);
            if (result.EvidenceKind != EvidenceKind.Live && p.RealizedNetProfitQuoteAtoms is not null)
                Add(field + ".realizedNetProfitQuoteAtoms", "profit_not_allowed");
            if (result.EvidenceKind == EvidenceKind.Synthetic && p.SimulatedNetProfitQuoteAtoms is not null)
                Add(field + ".simulatedNetProfitQuoteAtoms", "profit_not_allowed");
            if (p.RejectedCandidates is null) Add(field + ".rejectedCandidates", "required");
            else
            {
                var reasons = new HashSet<string>(StringComparer.Ordinal);
                for (int j = 0; j < p.RejectedCandidates.Count; j++)
                {
                    var rejection = p.RejectedCandidates[j];
                    var rejectionField = field + $".rejectedCandidates[{j}]";
                    if (rejection is null) { Add(rejectionField, "required"); continue; }
                    Identifier(rejectionField + ".reasonCode", rejection.ReasonCode);
                    Count(rejectionField + ".count", rejection.Count);
                    if (rejection.ReasonCode is { } reason && !reasons.Add(reason))
                        Add(rejectionField + ".reasonCode", "duplicate_reason");
                }
                if (p.CandidatesEvaluated is >= 0 && p.SelectedOrExecuted is >= 0
                    && p.RejectedCandidates.All(r => r?.Count is >= 0))
                {
                    long budget = p.CandidatesEvaluated.Value;
                    bool exceeded = p.SelectedOrExecuted.Value > budget;
                    if (!exceeded) budget -= p.SelectedOrExecuted.Value;
                    foreach (var rejection in p.RejectedCandidates)
                    {
                        if (exceeded || rejection!.Count!.Value > budget) { exceeded = true; break; }
                        budget -= rejection.Count.Value;
                    }
                    if (exceeded) Add(field, "candidate_count_exceeded");
                }
            }
        }
        return errors.AsReadOnly();
    }
}
