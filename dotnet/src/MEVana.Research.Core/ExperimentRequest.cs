using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace MEVana.Research.Core
{
    /// <summary>
    /// Immutable request DTO for experiment data queries.
    /// Retains raw invalid, null, offsets and ordering per spec.
    /// </summary>
    public sealed class ExperimentRequest
    {
        public ExperimentRequest(
            string? datasetId,
            string? strategyVersion,
            DateTimeOffset? windowStartUtc,
            DateTimeOffset? windowEndUtc,
            IReadOnlyList<string?>? tokenPair,
            IReadOnlyList<string?>? poolIds,
            long? minTradeSizeQuoteAtoms,
            long? maxTradeSizeQuoteAtoms)
        {
            DatasetId = datasetId;
            StrategyVersion = strategyVersion;
            WindowStartUtc = windowStartUtc;
            WindowEndUtc = windowEndUtc;

            // Defensive copy of tokenPair preserving nulls and order, allow null
            if (tokenPair == null)
            {
                TokenPair = null;
            }
            else
            {
                var tokenList = new List<string?>(tokenPair.Count);
                foreach (var token in tokenPair)
                {
                    tokenList.Add(token);
                }
                TokenPair = new ReadOnlyCollection<string?>(tokenList);
            }

            // Defensive copy of poolIds preserving nulls and order, allow null
            if (poolIds == null)
            {
                PoolIds = null;
            }
            else
            {
                var poolList = new List<string?>(poolIds.Count);
                foreach (var poolId in poolIds)
                {
                    poolList.Add(poolId);
                }
                PoolIds = new ReadOnlyCollection<string?>(poolList);
            }

            MinTradeSizeQuoteAtoms = minTradeSizeQuoteAtoms;
            MaxTradeSizeQuoteAtoms = maxTradeSizeQuoteAtoms;
        }

        public string? DatasetId { get; }
        public string? StrategyVersion { get; }
        public DateTimeOffset? WindowStartUtc { get; }
        public DateTimeOffset? WindowEndUtc { get; }
        public IReadOnlyList<string?>? TokenPair { get; }
        public IReadOnlyList<string?>? PoolIds { get; }
        public long? MinTradeSizeQuoteAtoms { get; }
        public long? MaxTradeSizeQuoteAtoms { get; }
    }
}
