# Re-Architecture 3 – AI-Augmented Trading Stack
Created: 2025-10-02

## Architectural Shifts with AI in the Loop
1. **Dedicated AI Orchestration Plane**
   - Introduce `ai-orchestration` service responsible for model lifecycle, feature pipelines, prompt templates, and inference routing.
   - All strategy, risk, and operator components call AI endpoints through signed RPC contracts; deterministic backstops remain untouched.

2. **Feature Fabric & Data Governance**
   - Stand up a real-time feature store (e.g., Redis Streams or Arrow Flight) built on normalized market events, portfolio state, and operator annotations.
   - Offline store (Parquet/Lake) powers training and audit trails; data contracts enforce schema evolution and PII scrubbing.

3. **Feedback & Learning Loop**
   - New topics: `ai.recommendations`, `ai.decisions`, `ai.feedback` capture inference responses, downstream actions, and realized outcomes.
   - `ai-simulator` service replays historical windows, validating new checkpoints against deterministic baselines before promotion.

4. **Operator Collaboration Layer**
   - Ops Console gains an AI Control Desk: approve/reject flows, rationale/explanation viewer, drift alerts, and policy override tools.
   - Human interventions produce labeled examples stored in a review ledger; these feed semi-supervised fine-tunes.

5. **Augmented Risk Pipeline**
   - Risk Gateway ingests AI outputs as advisory signals (e.g., anomaly score, suggested sizing multiplier) but enforces hard limits first.
   - AI policy guard service checks every inference against allowlists (expected ranges, monotonic constraints) and blocks unsafe outputs.

6. **Observability & Safety Envelopes**
   - Extend telemetry schema with `model_name`, `version`, `confidence`, `explanation_hash`; traces include inference latency and fallback decisions.
   - Drift detectors monitor input distributions, confidence decay, and outcome deltas; auto-fallback to baseline model when thresholds fail.

## Updated Logical Diagram
```
┌──────────────┐   ┌────────────────┐   ┌────────────────────┐   ┌─────────────────────┐
│ Data Sources │──▶│ Feed Ingestion │──▶│ Normalization Bus  │──▶│ Strategy Plugins     │
└──────────────┘   └────────────────┘   └────────────────────┘   └──────────┬──────────┘
                                                                       Signals │
                                                                               ▼
                                                                   ┌────────────────────┐
                                                                   │ AI Orchestration  │◀─────────┐
                                                                   └──────────┬─────────┘          │
                                                                          Advisories │            │
                                                                               ▼                    │
                                                       ┌────────────────────┐   Feedback           │
                                                       │ Risk & Policy Gate │──────────────┐      │
                                                       └──────────┬─────────┘              │      │
                                                              Orders │                     │      │
                                                                     ▼                     │      │
                                            ┌──────────────────┐  ┌──────────────┐        │      │
                                            │ Execution Router │──│ Venue Adapters│◀──────┘      │
                                            └────────┬─────────┘  └──────┬───────┘               │
                                                     │                 Fills                    │
                                                     ▼                                         │
                                         ┌────────────────────┐                                │
                                         │ Portfolio Service  │──────────────┐                 │
                                         └────────┬───────────┘              │                 │
                                                  │                         │                 │
                                                  ▼                         ▼                 │
                                 ┌────────────────────────┐   ┌────────────────────────┐      │
                                 │ Feature Store (Live)   │   │ Feedback Ledger / Lake │◀────┘
                                 └────────────────────────┘   └────────────────────────┘

Ops Console / AI Control Desk ⇄ AI Orchestration ⇄ Risk Gateway ⇄ Audit Log
```

## Service & Module Additions
```
services/
├── ai-orchestration/            # inference routing, model registry, prompt mgmt
├── ai-policy-guard/             # evaluates AI outputs against safety constraints
├── ai-simulator/                # replay & validation harness for new checkpoints
├── feature-store/               # real-time feature computation & serving
└── review-ledger/               # immutable log of operator decisions & feedback

crates/
├── ai-core/                     # shared feature builders, inference clients
├── ai-guards/                   # rule validators, monotonic constraints
├── ai-feedback/                 # feedback ingestion SDK, labeling helpers
└── explainability/              # SHAP-like explainers, structured rationales
```

## Lifecycle & Deployment Considerations
- **Model Promotion**: Train/fine-tune offline → validate via `ai-simulator` → shadow deploy with weighted traffic → canary → full rollout with automated rollback hooks.
- **Versioning**: Every inference carries `(model, revision, seed)` metadata; ops can freeze to last-known-good instantly.
- **Security**: Isolate models inside dedicated subnets, enforce signed requests from strategy/risk services, and log every inference request/response.
- **Testing**: CI includes deterministic replay runs, counterfactual analysis comparing AI vs baseline, and policy guard unit tests.

## Attribution
Authored by: Codex (GPT-5)
