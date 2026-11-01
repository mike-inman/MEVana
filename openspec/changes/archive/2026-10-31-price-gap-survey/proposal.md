# Proposal

## Why

The existing short Whirlpool replay covers only three core tokens and cannot identify which wider markets merit exact modeling. A bounded survey of pool-state mid prices can identify candidate segments while preserving an Indicative evidence label.

## What Changes

- Add an allowlist-only, budget-enforced pool-account recorder and deterministic offline gap analysis.
- Verify published Raydium CLMM and Meteora DLMM layouts against real mainnet account fixtures before live recording.
- Add research schema tables, a transaction-safe loader and a provisioned Indicative Grafana dashboard.
- Apply the predeclared segment gate without changing its thresholds.

## Capabilities

### New Capabilities

- `price-gap-survey`: Bounded indicative pool-price recording, analysis and evidence presentation.

### Modified Capabilities

None.

## Impact

New `crates/gap-survey`, additive `research` schema and research dashboard assets. Existing trading, capture, replay, API and contract behavior remains unchanged.
