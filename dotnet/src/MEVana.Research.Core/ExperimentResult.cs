namespace MEVana.Research.Core;

public enum EvidenceKind { Synthetic, Simulated, Live }
public enum ExperimentResultStatus { Completed, Failed, Unavailable }
public enum PolicyKind { FixedUniverse, SimpleRules, ModelRanking }

/// <summary>Raw rejection accounting; invalid values remain inspectable.</summary>
public sealed record RejectedCandidateCount(string? ReasonCode, long? Count);

public sealed class PolicyResult
{
    public PolicyResult(PolicyKind? kind, long? candidatesEvaluated, long? opportunitiesDetected,
        long? selectedOrExecuted, IReadOnlyList<RejectedCandidateCount?>? rejectedCandidates,
        long? missedCandidates, long? estimatedNetProfitQuoteAtoms,
        long? simulatedNetProfitQuoteAtoms, long? realizedNetProfitQuoteAtoms)
    {
        Kind = kind;
        CandidatesEvaluated = candidatesEvaluated;
        OpportunitiesDetected = opportunitiesDetected;
        SelectedOrExecuted = selectedOrExecuted;
        RejectedCandidates = rejectedCandidates is null ? null : Array.AsReadOnly(rejectedCandidates.ToArray());
        MissedCandidates = missedCandidates;
        EstimatedNetProfitQuoteAtoms = estimatedNetProfitQuoteAtoms;
        SimulatedNetProfitQuoteAtoms = simulatedNetProfitQuoteAtoms;
        RealizedNetProfitQuoteAtoms = realizedNetProfitQuoteAtoms;
    }

    public PolicyKind? Kind { get; }
    public long? CandidatesEvaluated { get; }
    public long? OpportunitiesDetected { get; }
    public long? SelectedOrExecuted { get; }
    public IReadOnlyList<RejectedCandidateCount?>? RejectedCandidates { get; }
    public long? MissedCandidates { get; }
    public long? EstimatedNetProfitQuoteAtoms { get; }
    public long? SimulatedNetProfitQuoteAtoms { get; }
    public long? RealizedNetProfitQuoteAtoms { get; }
}

/// <summary>Raw immutable submission. Use validators before consuming or canonicalizing.</summary>
public sealed class ExperimentResult
{
    public ExperimentResult(string? resultId, string? requestSha256, int? requestCanonicalizationVersion,
        string? datasetId, string? datasetMetadataVersion, string? strategyVersion, string? engineVersion,
        string? costModelVersion, DateTimeOffset? createdAtUtc, DateTimeOffset? windowStartUtc,
        DateTimeOffset? windowEndUtc, EvidenceKind? evidenceKind, ExperimentResultStatus? status,
        string? errorCode, IReadOnlyList<PolicyResult?>? policies)
    {
        ResultId = resultId;
        RequestSha256 = requestSha256;
        RequestCanonicalizationVersion = requestCanonicalizationVersion;
        DatasetId = datasetId;
        DatasetMetadataVersion = datasetMetadataVersion;
        StrategyVersion = strategyVersion;
        EngineVersion = engineVersion;
        CostModelVersion = costModelVersion;
        CreatedAtUtc = createdAtUtc;
        WindowStartUtc = windowStartUtc;
        WindowEndUtc = windowEndUtc;
        EvidenceKind = evidenceKind;
        Status = status;
        ErrorCode = errorCode;
        Policies = policies is null ? null : Array.AsReadOnly(policies.ToArray());
    }

    public string? ResultId { get; }
    public string? RequestSha256 { get; }
    public int? RequestCanonicalizationVersion { get; }
    public string? DatasetId { get; }
    public string? DatasetMetadataVersion { get; }
    public string? StrategyVersion { get; }
    public string? EngineVersion { get; }
    public string? CostModelVersion { get; }
    public DateTimeOffset? CreatedAtUtc { get; }
    public DateTimeOffset? WindowStartUtc { get; }
    public DateTimeOffset? WindowEndUtc { get; }
    public EvidenceKind? EvidenceKind { get; }
    public ExperimentResultStatus? Status { get; }
    public string? ErrorCode { get; }
    public IReadOnlyList<PolicyResult?>? Policies { get; }
}
