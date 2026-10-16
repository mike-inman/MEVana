using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using Xunit;
using MEVana.Research.Core;

namespace MEVana.Research.Tests
{
    public class ExperimentRequestContractTests
    {
        [Fact]
        public void Constructor_AllowsNullParameters()
        {
            var req = new ExperimentRequest(
                null, null, null, null, null, null, null, null);
            Assert.Null(req.DatasetId);
            Assert.Null(req.StrategyVersion);
            Assert.Null(req.WindowStartUtc);
            Assert.Null(req.WindowEndUtc);
            Assert.Null(req.TokenPair);
            Assert.Null(req.PoolIds);
            Assert.Null(req.MinTradeSizeQuoteAtoms);
            Assert.Null(req.MaxTradeSizeQuoteAtoms);
        }

        [Fact]
        public void Constructor_RetainsRawInvalidAndDuplicateValues()
        {
            var tokenPair = new List<string?> { null, "", "token1", "token2", "token1" };
            var poolIds = new List<string?> { null, "", "pool1", "pool2", "pool1" };

            var req = new ExperimentRequest(
                "dataset", "strategy", null, null, tokenPair, poolIds, -10L, 9223372036854775807L);

            Assert.Equal("dataset", req.DatasetId);
            Assert.Equal("strategy", req.StrategyVersion);
            Assert.Null(req.WindowStartUtc);
            Assert.Null(req.WindowEndUtc);

            Assert.NotNull(req.TokenPair);
            Assert.Equal(tokenPair.Count, req.TokenPair!.Count);
            for (int i = 0; i < tokenPair.Count; i++)
            {
                Assert.Equal(tokenPair[i], req.TokenPair[i]);
            }

            Assert.NotNull(req.PoolIds);
            Assert.Equal(poolIds.Count, req.PoolIds!.Count);
            for (int i = 0; i < poolIds.Count; i++)
            {
                Assert.Equal(poolIds[i], req.PoolIds[i]);
            }

            Assert.Equal(-10L, req.MinTradeSizeQuoteAtoms);
            Assert.Equal(9223372036854775807L, req.MaxTradeSizeQuoteAtoms);
        }

        [Fact]
        public void Constructor_PreservesOrderAndNullsInLists()
        {
            var tokens = new List<string?> { "base", null, "quote", "", "base" };
            var pools = new List<string?> { "p1", null, "p2", "", null };
            var req = new ExperimentRequest(
                "dataset", "strategy", DateTimeOffset.UtcNow, DateTimeOffset.UtcNow.AddHours(1), tokens, pools, 1L, 2L);

            Assert.NotNull(req.TokenPair);
            Assert.Equal(tokens.Count, req.TokenPair!.Count);
            for (int i = 0; i < tokens.Count; i++)
            {
                Assert.Equal(tokens[i], req.TokenPair[i]);
            }

            Assert.NotNull(req.PoolIds);
            Assert.Equal(pools.Count, req.PoolIds!.Count);
            for (int i = 0; i < pools.Count; i++)
            {
                Assert.Equal(pools[i], req.PoolIds[i]);
            }
        }

        [Fact]
        public void Constructor_DefensiveCopiesLists_ReadOnlySnapshots()
        {
            var tokens = new List<string?> { "tok1", "tok2" };
            var pools = new List<string?> { "pool1", "pool2" };

            var req = new ExperimentRequest(
                "dataset", "strategy", null, null, tokens, pools, 10L, 20L);

            // Modify the original lists after construction
            tokens[0] = "changed";
            pools[1] = "changed";

            // The request's lists should remain unchanged
            Assert.Equal("tok1", req.TokenPair![0]);
            Assert.Equal("tok2", req.TokenPair![1]);
            Assert.Equal("pool1", req.PoolIds![0]);
            Assert.Equal("pool2", req.PoolIds![1]);

            // The exposed lists should be read-only
            Assert.IsAssignableFrom<IReadOnlyList<string?>>(req.TokenPair);
            Assert.IsAssignableFrom<IReadOnlyList<string?>>(req.PoolIds);

            var tokenList = req.TokenPair as ReadOnlyCollection<string?>;
            var poolList = req.PoolIds as ReadOnlyCollection<string?>;

            Assert.NotNull(tokenList);
            Assert.NotNull(poolList);

            // Attempting to cast to List and modify should fail compilation (not tested here)
        }

        [Fact]
        public void Constructor_PreservesExactDateTimeOffsetValues()
        {
            var startOffset = new DateTimeOffset(2026,10,7,15,30,12,345, TimeSpan.FromHours(-7));
            var endOffset = new DateTimeOffset(2026,10,8,15,30,12,789, TimeSpan.FromHours(-7));
            var req = new ExperimentRequest("id", "ver", startOffset, endOffset, null, null, null, null);

            Assert.Equal(startOffset, req.WindowStartUtc);
            Assert.Equal(endOffset, req.WindowEndUtc);
        }

        [Fact]
        public void Constructor_AllowsNegativeAndZeroQuotes()
        {
            var req = new ExperimentRequest("id", "ver", null, null, null, null, 0L, -10L);
            Assert.Equal(0L, req.MinTradeSizeQuoteAtoms);
            Assert.Equal(-10L, req.MaxTradeSizeQuoteAtoms);
        }
    }
}
