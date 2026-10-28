using System.Globalization;
using System.Text;
using System.Text.Json;

namespace MEVana.Research.Core;

public sealed record CanonicalResult(int Version, string Content);

public static class ExperimentResultCanonicalizer
{
    public static CanonicalResult Canonicalize(ExperimentResult result)
    {
        ArgumentNullException.ThrowIfNull(result);
        if (ExperimentResultSyntaxValidator.Validate(result).Count != 0)
            throw new ArgumentException("Invalid typed result.", nameof(result));
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            void Time(string field, DateTimeOffset? value) => writer.WriteString(field,
                value!.Value.UtcDateTime.ToString("O", CultureInfo.InvariantCulture));
            void Number(string field, long? value)
            {
                if (value is { } number) writer.WriteNumber(field, number);
                else writer.WriteNull(field);
            }
            writer.WriteStartObject();
            writer.WriteNumber("canonicalizationVersion", 1);
            writer.WriteString("resultId", result.ResultId);
            writer.WriteString("requestSha256", result.RequestSha256);
            writer.WriteNumber("requestCanonicalizationVersion", result.RequestCanonicalizationVersion!.Value);
            writer.WriteString("datasetId", result.DatasetId);
            writer.WriteString("datasetMetadataVersion", result.DatasetMetadataVersion);
            writer.WriteString("strategyVersion", result.StrategyVersion);
            writer.WriteString("engineVersion", result.EngineVersion);
            writer.WriteString("costModelVersion", result.CostModelVersion);
            Time("createdAtUtc", result.CreatedAtUtc);
            Time("windowStartUtc", result.WindowStartUtc);
            Time("windowEndUtc", result.WindowEndUtc);
            writer.WriteString("evidenceKind", result.EvidenceKind!.Value.ToString());
            writer.WriteString("status", result.Status!.Value.ToString());
            writer.WriteString("errorCode", result.ErrorCode);
            if (result.Policies is null) writer.WriteNull("policies");
            else
            {
                writer.WriteStartArray("policies");
                foreach (var raw in result.Policies.OrderBy(p => p!.Kind))
                {
                    var p = raw!;
                    writer.WriteStartObject();
                    writer.WriteString("kind", p.Kind!.Value.ToString());
                    Number("candidatesEvaluated", p.CandidatesEvaluated);
                    Number("opportunitiesDetected", p.OpportunitiesDetected);
                    Number("selectedOrExecuted", p.SelectedOrExecuted);
                    writer.WriteStartArray("rejectedCandidates");
                    foreach (var rejection in p.RejectedCandidates!.OrderBy(r => r!.ReasonCode, StringComparer.Ordinal))
                    {
                        writer.WriteStartObject();
                        writer.WriteString("reasonCode", rejection!.ReasonCode);
                        Number("count", rejection.Count);
                        writer.WriteEndObject();
                    }
                    writer.WriteEndArray();
                    Number("missedCandidates", p.MissedCandidates);
                    Number("estimatedNetProfitQuoteAtoms", p.EstimatedNetProfitQuoteAtoms);
                    Number("simulatedNetProfitQuoteAtoms", p.SimulatedNetProfitQuoteAtoms);
                    Number("realizedNetProfitQuoteAtoms", p.RealizedNetProfitQuoteAtoms);
                    writer.WriteEndObject();
                }
                writer.WriteEndArray();
            }
            writer.WriteEndObject();
        }
        return new(1, Encoding.UTF8.GetString(stream.ToArray()));
    }
}
