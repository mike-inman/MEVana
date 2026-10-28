namespace MEVana.Research.Core;

public sealed record CanonicalExecutionOutcome(int Version, string Content);

public static class ExecutionOutcomeCanonicalizer
{
    public static CanonicalExecutionOutcome Canonicalize(ExecutionOutcome outcome)
    {
        ArgumentNullException.ThrowIfNull(outcome);
        if (ExecutionOutcomeSyntaxValidator.Validate(outcome).Count != 0) throw new ArgumentException("Invalid execution outcome.", nameof(outcome));
        var o = outcome;
        return new(1, ResearchContractJson.Write(w =>
        {
            w.WriteStartObject(); w.WriteNumber("canonicalizationVersion", 1); w.WriteNumber("schemaVersion", o.SchemaVersion!.Value);
            w.WriteString("outcomeId", o.OutcomeId); w.WriteString("idempotencyKey", o.IdempotencyKey);
            w.WriteString("datasetId", o.DatasetId); w.WriteString("datasetMetadataVersion", o.DatasetMetadataVersion); w.WriteString("strategyVersion", o.StrategyVersion);
            w.WriteString("policyKind", o.PolicyKind!.Value.ToString()); w.WriteString("rankingArtifactId", o.RankingArtifactId);
            ResearchContractJson.Number(w, "rankingEntryRank", o.RankingEntryRank); w.WriteString("experimentResultId", o.ExperimentResultId);
            w.WriteString("mode", o.Mode!.Value.ToString()); w.WriteString("evidenceKind", o.EvidenceKind!.Value.ToString());
            ResearchContractJson.Route(w, o.Route!); ResearchContractJson.Number(w, "inputSizeQuoteAtoms", o.InputSizeQuoteAtoms);
            ResearchContractJson.Number(w, "observedSlot", o.ObservedSlot);
            ResearchContractJson.Time(w, "detectedAtUtc", o.DetectedAtUtc); ResearchContractJson.Time(w, "decisionAtUtc", o.DecisionAtUtc);
            ResearchContractJson.Time(w, "submittedAtUtc", o.SubmittedAtUtc); ResearchContractJson.Number(w, "landedSlot", o.LandedSlot);
            ResearchContractJson.Time(w, "reconciledAtUtc", o.ReconciledAtUtc); w.WriteString("state", o.State!.Value.ToString());
            w.WriteString("reasonCode", o.ReasonCode); w.WriteString("transactionSignature", o.TransactionSignature);
            ResearchContractJson.Number(w, "estimatedNetProfitQuoteAtoms", o.EstimatedNetProfitQuoteAtoms);
            ResearchContractJson.Number(w, "simulatedNetProfitQuoteAtoms", o.SimulatedNetProfitQuoteAtoms);
            ResearchContractJson.Number(w, "realizedNetProfitQuoteAtoms", o.RealizedNetProfitQuoteAtoms);
            ResearchContractJson.Unsigned(w, "networkFeeQuoteAtoms", o.NetworkFeeQuoteAtoms);
            ResearchContractJson.Unsigned(w, "priorityFeeQuoteAtoms", o.PriorityFeeQuoteAtoms); ResearchContractJson.Unsigned(w, "tipQuoteAtoms", o.TipQuoteAtoms);
            w.WriteEndObject();
        }));
    }
}

public static class ExecutionOutcomeIdempotency
{
    /// <summary>Pure content comparison, not a durable reservation or execution guarantee.</summary>
    public static bool HasConflict(ExecutionOutcome first, ExecutionOutcome second)
    {
        var left = ExecutionOutcomeCanonicalizer.Canonicalize(first);
        var right = ExecutionOutcomeCanonicalizer.Canonicalize(second);
        return string.Equals(first.IdempotencyKey, second.IdempotencyKey, StringComparison.Ordinal)
            && !string.Equals(left.Content, right.Content, StringComparison.Ordinal);
    }
}
