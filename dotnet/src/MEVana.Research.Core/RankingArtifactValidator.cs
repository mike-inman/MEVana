namespace MEVana.Research.Core;

public static class RankingArtifactSyntaxValidator
{
    public static IReadOnlyList<ValidationIssue> Validate(RankingArtifact? artifact)
    {
        var e = new ContractIssues();
        if (artifact is null) { e.Add("artifact", "required"); return e.Finish(); }
        var a = artifact;
        e.Id("artifactId", a.ArtifactId); e.Schema(a.SchemaVersion);
        e.EnumValue("policyKind", a.PolicyKind); e.EnumValue("evidenceKind", a.EvidenceKind);
        e.Id("strategyVersion", a.StrategyVersion); e.Id("featureSetVersion", a.FeatureSetVersion);
        if (a.PolicyKind == PolicyKind.ModelRanking) e.Id("modelVersion", a.ModelVersion);
        else if (a.ModelVersion is not null) e.Add("modelVersion", "model_not_allowed");
        e.Id("datasetId", a.DatasetId); e.Id("datasetMetadataVersion", a.DatasetMetadataVersion);
        e.Id("sourceExperimentResultId", a.SourceExperimentResultId, false);
        e.Utc("trainingStartUtc", a.TrainingStartUtc); e.Utc("trainingEndUtc", a.TrainingEndUtc);
        if (a.TrainingStartUtc >= a.TrainingEndUtc) e.Add("trainingWindow", "invalid_window");
        e.Utc("asOfUtc", a.AsOfUtc); e.Utc("validFromUtc", a.ValidFromUtc); e.Utc("expiresAtUtc", a.ExpiresAtUtc);
        e.Range("observedSlot", a.ObservedSlot, 0, long.MaxValue, "invalid_count");
        if (a.TrainingEndUtc > a.AsOfUtc) e.Add("trainingWindow", "future_training");
        if (a.ValidFromUtc < a.AsOfUtc) e.Add("validFromUtc", "invalid_validity");
        if (a.ValidFromUtc is { } from && a.ExpiresAtUtc is { } expiry
            && (expiry <= from || expiry - from > TimeSpan.FromHours(24))) e.Add("expiresAtUtc", "invalid_validity");
        if (a.Entries is null) e.Add("entries", "required");
        else
        {
            if (a.Entries.Count is < 1 or > 1000) e.Add("entries", "invalid_entries");
            if (!a.Entries.Select(v => v?.Rank).OrderBy(v => v).SequenceEqual(Enumerable.Range(1, a.Entries.Count).Select(v => (int?)v)))
                e.Add("entries", "invalid_rank");
            var routes = new HashSet<string>(StringComparer.Ordinal);
            for (int i = 0; i < a.Entries.Count; i++)
            {
                string field = $"entries[{i}]"; var entry = a.Entries[i];
                if (entry is null) { e.Add(field, "required"); continue; }
                e.Range(field + ".rank", entry.Rank, 1, 1000, "invalid_rank");
                int before = e.Count; e.Route(entry.Route, field + ".route");
                if (e.Count == before)
                {
                    var route = entry.Route!;
                    var key = string.Join('|', route.TokenIds!) + "~" + string.Join('|', route.PoolIds!);
                    if (!routes.Add(key)) e.Add(field + ".route", "duplicate_route");
                }
                e.Range(field + ".minSizeQuoteAtoms", entry.MinSizeQuoteAtoms, 1, long.MaxValue, "invalid_size");
                e.Range(field + ".maxSizeQuoteAtoms", entry.MaxSizeQuoteAtoms, 1, long.MaxValue, "invalid_size");
                if (entry.MinSizeQuoteAtoms > entry.MaxSizeQuoteAtoms) e.Add(field + ".sizeRegion", "invalid_size");
                e.Range(field + ".monitoringPriority", entry.MonitoringPriority, 0, 100, "invalid_priority");
                e.Range(field + ".researchConfidenceBps", entry.ResearchConfidenceBps, 0, 10000, "invalid_basis_points");
                e.Range(field + ".calibratedSuccessProbabilityBps", entry.CalibratedSuccessProbabilityBps, 0, 10000, "invalid_basis_points", false);
                e.Range(field + ".sampleCount", entry.SampleCount, 0, long.MaxValue, "invalid_count");
            }
        }
        return e.Finish();
    }
}

public static class RankingArtifactValidator
{
    public static IReadOnlyList<ValidationIssue> Validate(RankingArtifact? artifact, ResearchCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        var syntax = RankingArtifactSyntaxValidator.Validate(artifact); if (syntax.Count != 0) return syntax;
        var a = artifact!; var e = new ContractIssues();
        var dataset = e.Dataset(catalog, a.DatasetId!, a.DatasetMetadataVersion!, a.StrategyVersion!);
        if (dataset is null) return e.Finish();
        if (a.TrainingStartUtc < dataset.CoverageStartUtc || a.TrainingEndUtc > dataset.CoverageEndUtc) e.Add("trainingWindow", "invalid_window");
        for (int i = 0; i < a.Entries!.Count; i++) e.CatalogRoute(a.Entries[i]!.Route!, dataset, a.Entries[i]!.MaxSizeQuoteAtoms!.Value, $"entries[{i}].route");
        return e.Finish();
    }
    public static IReadOnlyList<ValidationIssue> Validate(RankingArtifact? artifact, ResearchCatalog catalog, DateTimeOffset nowUtc)
    {
        var issues = Validate(artifact, catalog).ToList();
        if (nowUtc.Offset != TimeSpan.Zero) issues.Add(new("nowUtc", "invalid_utc"));
        else if (RankingArtifactSyntaxValidator.Validate(artifact).Count == 0)
        {
            if (nowUtc < artifact!.ValidFromUtc) issues.Add(new("validFromUtc", "not_yet_valid"));
            if (nowUtc >= artifact!.ExpiresAtUtc) issues.Add(new("expiresAtUtc", "expired"));
        }
        return issues.AsReadOnly();
    }
}
