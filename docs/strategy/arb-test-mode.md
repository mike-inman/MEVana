# Arbitrage Test Mode (Dry Run)

## Purpose
- Validate extractor v2 fields and sizing logic without sending bundles

## Behavior
- Consume v2 swaps; compute candidate sizes vs pool depth and fees
- Log “would trade” with net PnL and recommended tip=0 (canary)
- Skip trades with expected net <= 0

## Next
- Wire to Jito scaffold once acceptance/timing meets thresholds
