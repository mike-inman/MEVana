namespace MEVana.Research.Core;

/// <summary>Raw immutable research submission; validate before consumption.</summary>
public sealed class RankingEntry
{
    public RankingEntry(int? rank,
        ResearchRoute? route,
        long? minSizeQuoteAtoms,
        long? maxSizeQuoteAtoms,
        int? monitoringPriority,
        int? researchConfidenceBps,
        int? calibratedSuccessProbabilityBps,
        long? sampleCount,
        long? estimatedNetValueQuoteAtoms)
    {
        Rank = rank;
        Route = route;
        MinSizeQuoteAtoms = minSizeQuoteAtoms;
        MaxSizeQuoteAtoms = maxSizeQuoteAtoms;
        MonitoringPriority = monitoringPriority;
        ResearchConfidenceBps = researchConfidenceBps;
        CalibratedSuccessProbabilityBps = calibratedSuccessProbabilityBps;
        SampleCount = sampleCount;
        EstimatedNetValueQuoteAtoms = estimatedNetValueQuoteAtoms;
    }

    public int? Rank { get; }
    public ResearchRoute? Route { get; }
    public long? MinSizeQuoteAtoms { get; }
    public long? MaxSizeQuoteAtoms { get; }
    public int? MonitoringPriority { get; }
    public int? ResearchConfidenceBps { get; }
    public int? CalibratedSuccessProbabilityBps { get; }
    public long? SampleCount { get; }
    public long? EstimatedNetValueQuoteAtoms { get; }
}

/// <summary>Raw immutable research submission; validate before consumption.</summary>
public sealed class RankingArtifact
{
    public RankingArtifact(string? artifactId,
        int? schemaVersion,
        PolicyKind? policyKind,
        EvidenceKind? evidenceKind,
        string? strategyVersion,
        string? featureSetVersion,
        string? modelVersion,
        string? datasetId,
        string? datasetMetadataVersion,
        DateTimeOffset? trainingStartUtc,
        DateTimeOffset? trainingEndUtc,
        string? sourceExperimentResultId,
        DateTimeOffset? asOfUtc,
        long? observedSlot,
        DateTimeOffset? validFromUtc,
        DateTimeOffset? expiresAtUtc,
        IReadOnlyList<RankingEntry?>? entries)
    {
        ArtifactId = artifactId;
        SchemaVersion = schemaVersion;
        PolicyKind = policyKind;
        EvidenceKind = evidenceKind;
        StrategyVersion = strategyVersion;
        FeatureSetVersion = featureSetVersion;
        ModelVersion = modelVersion;
        DatasetId = datasetId;
        DatasetMetadataVersion = datasetMetadataVersion;
        TrainingStartUtc = trainingStartUtc;
        TrainingEndUtc = trainingEndUtc;
        SourceExperimentResultId = sourceExperimentResultId;
        AsOfUtc = asOfUtc;
        ObservedSlot = observedSlot;
        ValidFromUtc = validFromUtc;
        ExpiresAtUtc = expiresAtUtc;
        Entries = entries is null ? null : Array.AsReadOnly(entries.ToArray());
    }

    public string? ArtifactId { get; }
    public int? SchemaVersion { get; }
    public PolicyKind? PolicyKind { get; }
    public EvidenceKind? EvidenceKind { get; }
    public string? StrategyVersion { get; }
    public string? FeatureSetVersion { get; }
    public string? ModelVersion { get; }
    public string? DatasetId { get; }
    public string? DatasetMetadataVersion { get; }
    public DateTimeOffset? TrainingStartUtc { get; }
    public DateTimeOffset? TrainingEndUtc { get; }
    public string? SourceExperimentResultId { get; }
    public DateTimeOffset? AsOfUtc { get; }
    public long? ObservedSlot { get; }
    public DateTimeOffset? ValidFromUtc { get; }
    public DateTimeOffset? ExpiresAtUtc { get; }
    public IReadOnlyList<RankingEntry?>? Entries { get; }
    public string Kind => "WATCH";
}
