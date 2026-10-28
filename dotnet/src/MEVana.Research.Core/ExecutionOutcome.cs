namespace MEVana.Research.Core;

public enum ExecutionMode { Shadow, Paper, LiveSubmission }
public enum ExecutionOutcomeState { Rejected, NotSubmitted, Landed, Failed, Dropped, Expired }

/// <summary>Raw immutable research submission; validate before consumption.</summary>
public sealed class ExecutionOutcome
{
    public ExecutionOutcome(string? outcomeId,
        int? schemaVersion,
        string? idempotencyKey,
        string? datasetId,
        string? datasetMetadataVersion,
        string? strategyVersion,
        PolicyKind? policyKind,
        string? rankingArtifactId,
        int? rankingEntryRank,
        string? experimentResultId,
        ExecutionMode? mode,
        EvidenceKind? evidenceKind,
        ResearchRoute? route,
        long? inputSizeQuoteAtoms,
        long? observedSlot,
        DateTimeOffset? detectedAtUtc,
        DateTimeOffset? decisionAtUtc,
        DateTimeOffset? submittedAtUtc,
        long? landedSlot,
        DateTimeOffset? reconciledAtUtc,
        ExecutionOutcomeState? state,
        string? reasonCode,
        string? transactionSignature,
        long? estimatedNetProfitQuoteAtoms,
        long? simulatedNetProfitQuoteAtoms,
        long? realizedNetProfitQuoteAtoms,
        ulong? networkFeeQuoteAtoms,
        ulong? priorityFeeQuoteAtoms,
        ulong? tipQuoteAtoms)
    {
        OutcomeId = outcomeId;
        SchemaVersion = schemaVersion;
        IdempotencyKey = idempotencyKey;
        DatasetId = datasetId;
        DatasetMetadataVersion = datasetMetadataVersion;
        StrategyVersion = strategyVersion;
        PolicyKind = policyKind;
        RankingArtifactId = rankingArtifactId;
        RankingEntryRank = rankingEntryRank;
        ExperimentResultId = experimentResultId;
        Mode = mode;
        EvidenceKind = evidenceKind;
        Route = route;
        InputSizeQuoteAtoms = inputSizeQuoteAtoms;
        ObservedSlot = observedSlot;
        DetectedAtUtc = detectedAtUtc;
        DecisionAtUtc = decisionAtUtc;
        SubmittedAtUtc = submittedAtUtc;
        LandedSlot = landedSlot;
        ReconciledAtUtc = reconciledAtUtc;
        State = state;
        ReasonCode = reasonCode;
        TransactionSignature = transactionSignature;
        EstimatedNetProfitQuoteAtoms = estimatedNetProfitQuoteAtoms;
        SimulatedNetProfitQuoteAtoms = simulatedNetProfitQuoteAtoms;
        RealizedNetProfitQuoteAtoms = realizedNetProfitQuoteAtoms;
        NetworkFeeQuoteAtoms = networkFeeQuoteAtoms;
        PriorityFeeQuoteAtoms = priorityFeeQuoteAtoms;
        TipQuoteAtoms = tipQuoteAtoms;
    }

    public string? OutcomeId { get; }
    public int? SchemaVersion { get; }
    public string? IdempotencyKey { get; }
    public string? DatasetId { get; }
    public string? DatasetMetadataVersion { get; }
    public string? StrategyVersion { get; }
    public PolicyKind? PolicyKind { get; }
    public string? RankingArtifactId { get; }
    public int? RankingEntryRank { get; }
    public string? ExperimentResultId { get; }
    public ExecutionMode? Mode { get; }
    public EvidenceKind? EvidenceKind { get; }
    public ResearchRoute? Route { get; }
    public long? InputSizeQuoteAtoms { get; }
    public long? ObservedSlot { get; }
    public DateTimeOffset? DetectedAtUtc { get; }
    public DateTimeOffset? DecisionAtUtc { get; }
    public DateTimeOffset? SubmittedAtUtc { get; }
    public long? LandedSlot { get; }
    public DateTimeOffset? ReconciledAtUtc { get; }
    public ExecutionOutcomeState? State { get; }
    public string? ReasonCode { get; }
    public string? TransactionSignature { get; }
    public long? EstimatedNetProfitQuoteAtoms { get; }
    public long? SimulatedNetProfitQuoteAtoms { get; }
    public long? RealizedNetProfitQuoteAtoms { get; }
    public ulong? NetworkFeeQuoteAtoms { get; }
    public ulong? PriorityFeeQuoteAtoms { get; }
    public ulong? TipQuoteAtoms { get; }
}
