using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace MEVana.Research.Core
{
    public sealed class DiscoveryAsset
    {
        public DiscoveryAsset(string? chainId, string? mintAddress)
        {
            ChainId = chainId;
            MintAddress = mintAddress;
        }

        public string? ChainId { get; }
        public string? MintAddress { get; }
    }

    public sealed class DiscoveryVenue
    {
        public DiscoveryVenue(string? dexId, string? protocolVariant, string? poolAddress)
        {
            DexId = dexId;
            ProtocolVariant = protocolVariant;
            PoolAddress = poolAddress;
        }

        public string? DexId { get; }
        public string? ProtocolVariant { get; }
        public string? PoolAddress { get; }
    }

    public sealed class DiscoveryObservation
    {
        public DiscoveryObservation(string? sourceId, string? captureId,
            DateTimeOffset? observedAtUtc, ulong? slot)
        {
            SourceId = sourceId;
            CaptureId = captureId;
            ObservedAtUtc = observedAtUtc;
            Slot = slot;
        }

        public string? SourceId { get; }
        public string? CaptureId { get; }
        public DateTimeOffset? ObservedAtUtc { get; }
        public ulong? Slot { get; }
    }

    public sealed class DiscoveryQuote
    {
        public DiscoveryQuote(
            DiscoveryAsset? inputAsset, DiscoveryAsset? outputAsset,
            ulong? inputAtoms, ulong? outputAtoms,
            DiscoveryVenue? venue,
            DiscoveryObservation? observation,
            bool? swapFeesIncluded)
        {
            InputAsset = inputAsset;
            OutputAsset = outputAsset;
            InputAtoms = inputAtoms;
            OutputAtoms = outputAtoms;
            Venue = venue;
            Observation = observation;
            SwapFeesIncluded = swapFeesIncluded;
        }

        public DiscoveryAsset? InputAsset { get; }
        public DiscoveryAsset? OutputAsset { get; }
        public ulong? InputAtoms { get; }
        public ulong? OutputAtoms { get; }
        public DiscoveryVenue? Venue { get; }
        public DiscoveryObservation? Observation { get; }
        public bool? SwapFeesIncluded { get; }
    }

    public sealed class DiscoveryCost
    {
        public DiscoveryCost(
            string? kind, DiscoveryAsset? asset,
            ulong? amountAtoms, DiscoveryObservation? observation)
        {
            Kind = kind;
            Asset = asset;
            AmountAtoms = amountAtoms;
            Observation = observation;
        }

        public string? Kind { get; }
        public DiscoveryAsset? Asset { get; }
        public ulong? AmountAtoms { get; }
        public DiscoveryObservation? Observation { get; }
    }
}
