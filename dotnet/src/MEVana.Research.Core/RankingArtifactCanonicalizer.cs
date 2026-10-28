namespace MEVana.Research.Core;

public sealed record CanonicalRankingArtifact(int Version, string Content);

public static class RankingArtifactCanonicalizer
{
    public static CanonicalRankingArtifact Canonicalize(RankingArtifact artifact)
    {
        ArgumentNullException.ThrowIfNull(artifact);
        if (RankingArtifactSyntaxValidator.Validate(artifact).Count != 0) throw new ArgumentException("Invalid ranking artifact.", nameof(artifact));
        var a = artifact;
        return new(1, ResearchContractJson.Write(w =>
        {
            w.WriteStartObject(); w.WriteNumber("canonicalizationVersion", 1); w.WriteNumber("schemaVersion", a.SchemaVersion!.Value);
            w.WriteString("kind", a.Kind); w.WriteString("artifactId", a.ArtifactId);
            w.WriteString("policyKind", a.PolicyKind!.Value.ToString()); w.WriteString("evidenceKind", a.EvidenceKind!.Value.ToString());
            w.WriteString("strategyVersion", a.StrategyVersion); w.WriteString("featureSetVersion", a.FeatureSetVersion);
            w.WriteString("modelVersion", a.ModelVersion); w.WriteString("datasetId", a.DatasetId); w.WriteString("datasetMetadataVersion", a.DatasetMetadataVersion);
            ResearchContractJson.Time(w, "trainingStartUtc", a.TrainingStartUtc); ResearchContractJson.Time(w, "trainingEndUtc", a.TrainingEndUtc);
            w.WriteString("sourceExperimentResultId", a.SourceExperimentResultId); ResearchContractJson.Time(w, "asOfUtc", a.AsOfUtc);
            ResearchContractJson.Number(w, "observedSlot", a.ObservedSlot); ResearchContractJson.Time(w, "validFromUtc", a.ValidFromUtc);
            ResearchContractJson.Time(w, "expiresAtUtc", a.ExpiresAtUtc);
            w.WriteStartArray("entries");
            foreach (var raw in a.Entries!.OrderBy(v => v!.Rank))
            {
                var entry = raw!; w.WriteStartObject(); ResearchContractJson.Number(w, "rank", entry.Rank);
                ResearchContractJson.Route(w, entry.Route!);
                ResearchContractJson.Number(w, "minSizeQuoteAtoms", entry.MinSizeQuoteAtoms); ResearchContractJson.Number(w, "maxSizeQuoteAtoms", entry.MaxSizeQuoteAtoms);
                ResearchContractJson.Number(w, "monitoringPriority", entry.MonitoringPriority); ResearchContractJson.Number(w, "researchConfidenceBps", entry.ResearchConfidenceBps);
                ResearchContractJson.Number(w, "calibratedSuccessProbabilityBps", entry.CalibratedSuccessProbabilityBps);
                ResearchContractJson.Number(w, "sampleCount", entry.SampleCount); ResearchContractJson.Number(w, "estimatedNetValueQuoteAtoms", entry.EstimatedNetValueQuoteAtoms);
                w.WriteEndObject();
            }
            w.WriteEndArray(); w.WriteEndObject();
        }));
    }
}
