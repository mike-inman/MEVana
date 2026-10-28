namespace MEVana.Research.Core;

public static class ExecutionOutcomeSyntaxValidator
{
    const string Base58Alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    public static IReadOnlyList<ValidationIssue> Validate(ExecutionOutcome? outcome)
    {
        var e = new ContractIssues();
        if (outcome is null) { e.Add("outcome", "required"); return e.Finish(); }
        var o = outcome;
        e.Id("outcomeId", o.OutcomeId); e.Schema(o.SchemaVersion); e.Id("idempotencyKey", o.IdempotencyKey);
        e.Id("datasetId", o.DatasetId); e.Id("datasetMetadataVersion", o.DatasetMetadataVersion); e.Id("strategyVersion", o.StrategyVersion);
        e.EnumValue("policyKind", o.PolicyKind); e.Id("rankingArtifactId", o.RankingArtifactId, false);
        if ((o.RankingArtifactId is null) != (o.RankingEntryRank is null)) e.Add("rankingLink", "invalid_ranking_link");
        e.Range("rankingEntryRank", o.RankingEntryRank, 1, 1000, "invalid_rank", false);
        e.Id("experimentResultId", o.ExperimentResultId, false);
        e.EnumValue("mode", o.Mode); e.EnumValue("evidenceKind", o.EvidenceKind); e.EnumValue("state", o.State);
        if (o.Mode == ExecutionMode.LiveSubmission && o.EvidenceKind != EvidenceKind.Live
            || o.EvidenceKind == EvidenceKind.Live && o.Mode != ExecutionMode.LiveSubmission) e.Add("evidenceKind", "incompatible_evidence");
        e.Route(o.Route, "route");
        e.Range("inputSizeQuoteAtoms", o.InputSizeQuoteAtoms, 1, long.MaxValue, "invalid_size");
        e.Range("observedSlot", o.ObservedSlot, 0, long.MaxValue, "invalid_count");
        e.Utc("detectedAtUtc", o.DetectedAtUtc); e.Utc("decisionAtUtc", o.DecisionAtUtc);
        e.Utc("submittedAtUtc", o.SubmittedAtUtc, false); e.Utc("reconciledAtUtc", o.ReconciledAtUtc, false);
        DateTimeOffset? previous = null;
        foreach (var (field, time) in new[] { ("detectedAtUtc", o.DetectedAtUtc), ("decisionAtUtc", o.DecisionAtUtc),
            ("submittedAtUtc", o.SubmittedAtUtc), ("reconciledAtUtc", o.ReconciledAtUtc) })
        {
            if (time is null) continue;
            if (previous is not null && time < previous) e.Add(field, "non_monotonic_time");
            previous = time;
        }
        bool submitted = o.State is ExecutionOutcomeState.Landed or ExecutionOutcomeState.Failed or ExecutionOutcomeState.Dropped;
        bool landed = o.State is ExecutionOutcomeState.Landed or ExecutionOutcomeState.Failed;
        bool reasonRequired = o.State is ExecutionOutcomeState.Rejected or ExecutionOutcomeState.Failed or ExecutionOutcomeState.Dropped or ExecutionOutcomeState.Expired;
        e.Id("reasonCode", o.ReasonCode, reasonRequired);
        if (submitted)
        {
            if (o.Mode is not (ExecutionMode.Paper or ExecutionMode.LiveSubmission)) e.Add("mode", "submission_not_allowed");
            if (o.SubmittedAtUtc is null) e.Add("submittedAtUtc", "required");
        }
        else if (o.SubmittedAtUtc is not null) e.Add("submittedAtUtc", "submission_not_allowed");
        if (landed)
        {
            e.Range("landedSlot", o.LandedSlot, 0, long.MaxValue, "invalid_slot");
            if (o.LandedSlot < o.ObservedSlot) e.Add("landedSlot", "invalid_slot");
        }
        else if (o.LandedSlot is not null) e.Add("landedSlot", "invalid_slot");
        if (submitted && o.Mode == ExecutionMode.LiveSubmission)
        {
            if (o.TransactionSignature is null) e.Add("transactionSignature", "required");
            else if (o.TransactionSignature.Length is < 64 or > 88 || !o.TransactionSignature.All(Base58Alphabet.Contains))
                e.Add("transactionSignature", "invalid_signature");
        }
        else if (o.TransactionSignature is not null) e.Add("transactionSignature", "signature_not_allowed");
        if (o.EvidenceKind == EvidenceKind.Synthetic && o.SimulatedNetProfitQuoteAtoms is not null)
            e.Add("simulatedNetProfitQuoteAtoms", "profit_not_allowed");
        if (o.RealizedNetProfitQuoteAtoms is not null)
        {
            if (o.Mode != ExecutionMode.LiveSubmission || !landed) e.Add("realizedNetProfitQuoteAtoms", "profit_not_allowed");
            if (o.ReconciledAtUtc is null) e.Add("reconciledAtUtc", "required");
        }
        if (!submitted && (o.NetworkFeeQuoteAtoms is not null || o.PriorityFeeQuoteAtoms is not null || o.TipQuoteAtoms is not null))
            e.Add("costs", "costs_not_allowed");
        return e.Finish();
    }
}

public static class ExecutionOutcomeValidator
{
    public static IReadOnlyList<ValidationIssue> Validate(ExecutionOutcome? outcome, ResearchCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        var syntax = ExecutionOutcomeSyntaxValidator.Validate(outcome); if (syntax.Count != 0) return syntax;
        var o = outcome!; var e = new ContractIssues();
        var dataset = e.Dataset(catalog, o.DatasetId!, o.DatasetMetadataVersion!, o.StrategyVersion!);
        if (dataset is null) return e.Finish();
        if (o.DetectedAtUtc < dataset.CoverageStartUtc || o.DetectedAtUtc > dataset.CoverageEndUtc) e.Add("detectedAtUtc", "invalid_window");
        e.CatalogRoute(o.Route!, dataset, o.InputSizeQuoteAtoms!.Value, "route");
        return e.Finish();
    }
}
