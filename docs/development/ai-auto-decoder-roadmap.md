# AI Auto-Decoder Roadmap
Last updated: 2025-09-15

This plan enables autonomous growth of supported DEX decoders while preserving safety and quality.

## Pipeline Overview

1) Candidate Decoders Folder
- Repository contains a `decoders-candidates/` (or `profiles/candidates/`) directory.
- Humans or AI add new profiles/IDLs via PRs to this folder.

2) CI Validation
- On PR: run codegen and build the workspace.
- Execute unit tests with fixtures (golden JSON outputs).
- Run a simulated extractor over a recorded raw stream (pcap or Kafka export) and compute metrics:
  - Recognized instruction rate per program
  - Extract success ratio (non-empty normalized swaps)
  - Price plausibility checks (e.g., ratio bounds)

3) Auto-Promotion Criteria
- If metrics exceed thresholds (configurable), a bot opens a follow-up PR to move the new decoder(s) from `candidates/` to `profiles/`.
- Otherwise, the PR posts a report and suggested adjustments (e.g., missing roles, wrong account order).

4) Watchdog Service
- A lightweight daemon monitors incoming raw txs (Kafka topic) and tracks undecoded instruction spikes.
- On anomalies, it opens a “learn” issue with:
  - Program ID and sample instructions
  - Inferred role hints from on-chain account positions
  - Links to any existing IDL references

## Implementation Notes

- Codegen already prefers the profile PID over IDL metadata (done).
- Golden fixtures: store minimal JSON tx snippets and expected NormalizedSwapV1 outputs.
- Simulation: replay a bounded sample of raw prost messages into the extractor for offline scoring.
- Metrics: export Prometheus counters in CI logs and attach summary as a PR comment.

## Rollout

- Phase 1: Manual PRs into `candidates/` + CI scoring + maintainers review.
- Phase 2: Semi-automated template generation from IDL + human confirmation of role mappings.
- Phase 3: Fully automated role inference for common Anchor patterns; human approval gate.


