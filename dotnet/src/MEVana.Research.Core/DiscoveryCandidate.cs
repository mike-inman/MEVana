using System.Collections.Generic;
using System.Collections.ObjectModel;
using System;

namespace MEVana.Research.Core
{
    public sealed class DiscoveryCandidate
    {
        public DiscoveryCandidate(
            string? candidateId,
            DiscoveryQuote? firstLeg,
            DiscoveryQuote? secondLeg,
            IReadOnlyList<DiscoveryCost?>? additionalCosts,
            IReadOnlyList<string?>? evidenceIssues)
        {
            CandidateId = candidateId;
            FirstLeg = firstLeg;
            SecondLeg = secondLeg;

            AdditionalCosts = additionalCosts is null ? null : new ReadOnlyCollection<DiscoveryCost?>(
                new List<DiscoveryCost?>(additionalCosts));

            EvidenceIssues = evidenceIssues is null ? null : new ReadOnlyCollection<string?>(
                new List<string?>(evidenceIssues));
        }

        public string? CandidateId { get; }
        public DiscoveryQuote? FirstLeg { get; }
        public DiscoveryQuote? SecondLeg { get; }
        public IReadOnlyList<DiscoveryCost?>? AdditionalCosts { get; }
        public IReadOnlyList<string?>? EvidenceIssues { get; }
    }
}
