using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using Xunit;
using MEVana.Research.Core;

namespace MEVana.Research.Tests
{
    public class DiscoveryContractTests
    {
        [Fact]
        public void DiscoveryAsset_PreservesNullsAndValues()
        {
            var a1 = new DiscoveryAsset(null, null);
            var a2 = new DiscoveryAsset("ChainA", "MintX");

            Assert.Null(a1.ChainId);
            Assert.Null(a1.MintAddress);

            Assert.Equal("ChainA", a2.ChainId);
            Assert.Equal("MintX", a2.MintAddress);
        }

        [Fact]
        public void DiscoveryVenue_PreservesNullsAndValues()
        {
            var v1 = new DiscoveryVenue(null, null, null);
            var v2 = new DiscoveryVenue("Dex1", "VarA", "Pool123");

            Assert.Null(v1.DexId);
            Assert.Null(v1.ProtocolVariant);
            Assert.Null(v1.PoolAddress);

            Assert.Equal("Dex1", v2.DexId);
            Assert.Equal("VarA", v2.ProtocolVariant);
            Assert.Equal("Pool123", v2.PoolAddress);
        }

        [Fact]
        public void DiscoveryObservation_PreservesAllProperties()
        {
            var utcNow = DateTimeOffset.UtcNow;
            ulong? slot = 12345;
            
            var obs1 = new DiscoveryObservation(null, null, null, null);
            var obs2 = new DiscoveryObservation("src", "cap", utcNow, slot);

            Assert.Null(obs1.SourceId);
            Assert.Null(obs1.CaptureId);
            Assert.Null(obs1.ObservedAtUtc);
            Assert.Null(obs1.Slot);

            Assert.Equal("src", obs2.SourceId);
            Assert.Equal("cap", obs2.CaptureId);
            Assert.Equal(utcNow, obs2.ObservedAtUtc);
            Assert.Equal(slot, obs2.Slot);
        }

        [Fact]
        public void DiscoveryQuote_PreservesAllProperties()
        {
            var asset1 = new DiscoveryAsset("C1", "M1");
            var asset2 = new DiscoveryAsset("C2", "M2");
            var venue = new DiscoveryVenue("Dex", "Var", "Pool");
            var obs = new DiscoveryObservation("S", "C", DateTimeOffset.UtcNow, 9999);

            var quote = new DiscoveryQuote(
                asset1, asset2, 0, ulong.MaxValue, venue, obs, true);

            Assert.Equal(asset1, quote.InputAsset);
            Assert.Equal(asset2, quote.OutputAsset);
            Assert.Equal((ulong?)0, quote.InputAtoms);
            Assert.Equal((ulong?)ulong.MaxValue, quote.OutputAtoms);
            Assert.Equal(venue, quote.Venue);
            Assert.Equal(obs, quote.Observation);
            Assert.True(quote.SwapFeesIncluded);

            var quoteNullFees = new DiscoveryQuote(null, null, null, null, null, null, null);
            Assert.Null(quoteNullFees.SwapFeesIncluded);
        }

        [Fact]
        public void DiscoveryCost_PreservesAllProperties()
        {
            var asset = new DiscoveryAsset("ChainX", "MintX");
            var obs = new DiscoveryObservation("src", "cap", DateTimeOffset.UtcNow, 123);

            var cost = new DiscoveryCost("fee", asset, 0, obs);

            Assert.Equal("fee", cost.Kind);
            Assert.Equal(asset, cost.Asset);
            Assert.Equal((ulong?)0, cost.AmountAtoms);
            Assert.Equal(obs, cost.Observation);

            var costNulls = new DiscoveryCost(null, null, null, null);
            Assert.Null(costNulls.Kind);
            Assert.Null(costNulls.Asset);
            Assert.Null(costNulls.AmountAtoms);
            Assert.Null(costNulls.Observation);
        }

        [Fact]
        public void DiscoveryCandidate_PreservesPropertiesAndCopiesCollections()
        {
            var cost1 = new DiscoveryCost("k1", new DiscoveryAsset("c1", "m1"), 1, null);
            var cost2 = new DiscoveryCost(null, null, ulong.MaxValue, null);

            var addCosts = new List<DiscoveryCost?> { cost1, null, cost2 };
            var issues = new List<string?> { "issue1", null, "issue3"};

            var firstLeg = new DiscoveryQuote(
                null, null, null, null, null, null, null);
            var secondLeg = new DiscoveryQuote(
                null, null, null, null, null, null, null);

            var candidate = new DiscoveryCandidate(
                "cid",
                firstLeg,
                secondLeg,
                addCosts,
                issues);

            Assert.Equal("cid", candidate.CandidateId);
            Assert.Equal(firstLeg, candidate.FirstLeg);
            Assert.Equal(secondLeg, candidate.SecondLeg);

            // Collections are not the same instance; they are defensive copies
            Assert.NotSame(addCosts, candidate.AdditionalCosts);
            Assert.NotSame(issues, candidate.EvidenceIssues);

            // Contents are the same and order preserved, including nulls
            Assert.Equal(addCosts, candidate.AdditionalCosts);
            Assert.Equal(issues, candidate.EvidenceIssues);

            // Mutate original collections after construction should not affect candidate properties
            addCosts.Clear();
            issues[0] = "changed";
            Assert.NotEqual(addCosts, candidate.AdditionalCosts);
            Assert.NotEqual(issues, candidate.EvidenceIssues);

            // Null collections preserved
            var candidateNulls = new DiscoveryCandidate(null, null, null, null, null);
            Assert.Null(candidateNulls.AdditionalCosts);
            Assert.Null(candidateNulls.EvidenceIssues);
        }

        [Fact]
        public void DiscoveryCandidate_ExposesReadOnlyCollection_ForAdditionalCostsAndEvidenceIssues()
        {
            var cost = new DiscoveryCost("k", null, 1, null);
            var costsList = new List<DiscoveryCost?> { cost };
            var issuesList = new List<string?> { "issue" };

            var candidate = new DiscoveryCandidate("id", null, null, costsList, issuesList);

            Assert.IsAssignableFrom<IReadOnlyList<DiscoveryCost?>>(candidate.AdditionalCosts!);
            Assert.IsAssignableFrom<IReadOnlyList<string?>>(candidate.EvidenceIssues!);

            // Try cast to ReadOnlyCollection to test immutability via test cast
            var costsRO = candidate.AdditionalCosts as ReadOnlyCollection<DiscoveryCost?>;
            Assert.NotNull(costsRO);

            var issuesRO = candidate.EvidenceIssues as ReadOnlyCollection<string?>;
            Assert.NotNull(issuesRO);
        }
    }
}
