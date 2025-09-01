Project Board Scaffold
======================

Purpose
-------
Lightweight workflow structure for tracking progress across observability, performance, reliability, and tooling objectives without introducing heavy process. Designed for a single maintainer (you) but future‑proofed for collaborators.

Board Columns (Proposed)
------------------------
1. Inbox / Triage
   - Newly created issues not yet categorized or sized.
   - SLA: Triage within 24h; either close, merge into existing, or move to Backlog.
2. Backlog
   - Accepted issues not yet scheduled.
   - Ordered roughly by priority labels (e.g. `priority-early`).
3. Ready
   - Well-specified, acceptance criteria clear, no external dependency.
   - Pull from here when starting new work.
4. In Progress
   - Actively being implemented (limit WIP to 2 to maintain focus).
5. Review / Verify
   - Implementation complete; perform local validation (build, tests, lint, perf spot-check) before moving to Done.
6. Done
   - Merged & validated; update related docs and remove from Backlog lists.
7. Parking Lot / Blocked (optional swimlane)
   - Blocked on external dependency or pending design decision.

Status Mapping (Label Strategy)
-------------------------------
Because only one contributor is active, explicit status labels are optional. If added later, suggested labels:
- `status:ready`
- `status:in-progress`
- `status:review`
- `status:blocked`
- `status:done` (applied automatically via automation or manually upon merge)

Current Issues → Suggested Milestones (Logical Phases)
-----------------------------------------------------
Milestone M1 – Observability Foundation
- #1 Export max edge factor
- #2 Gauge counts (edges / tokens / adjacency)
- #3 Structured JSON signal logging
- #4 Tracing spans for scan phases
- #5 Memory / allocator stats
- #18 Golden file tests for metrics output

Milestone M2 – Sizing & Performance Optimization
- #6 Adaptive sizing
- #7 Depth / slippage-aware sizing solver
- #8 Path cache + invalidation
- #9 Liquidity/volume-based edge filtering
- #10 Multi-thread / sharded detector

Milestone M3 – Resilience & Reliability
- #11 Circuit breakers & stale edge detection
- #12 Config hot-reload trigger
- #13 Per-venue health metrics & cooldown
- #14 Staleness-based edge auto-prune

Milestone M4 – Tooling, Backtest, Bench & Fuzz
- #15 Backtest & replay harness
- #16 Scenario fuzzing (extreme price/fee combos)
- #17 Criterion bench suite

Prioritization Heuristics
-------------------------
1. Unblock future design surface: implement low-risk observability (M1) before deeper performance refactors to gain measurement fidelity.
2. Lay sizing groundwork (M2) before heavy parallelism (#10) to avoid duplicating optimization work across shards.
3. Add reliability gates (M3) once measurement + performance baseline exists.
4. Tooling (M4) continuously improves confidence; schedule early slices if risk rises.

Automation Ideas (Optional Future Enhancements)
----------------------------------------------
- GitHub Actions workflow labeling PRs touching `crates/arb-engine` with `performance` or `observability` based on diff keywords.
- Auto-move issues to `Review / Verify` when a PR with `Fixes #X` is merged.
- Nightly benchmark job posts comment to #17 with latest latency percentiles and diff vs. previous run.

Definition of Ready Checklist (for moving issue to Ready)
--------------------------------------------------------
- Clear acceptance criteria.
- Performance impact expectation (if applicable) stated.
- Metrics additions (if any) named explicitly.
- Feature gating strategy decided (always-on vs. behind feature flag).

Definition of Done Checklist
----------------------------
- Code merged to `main`.
- All acceptance criteria satisfied.
- Build + tests pass locally.
- Metrics / docs updated (README or `docs/` subsection).
- Backlog / roadmap docs pruned or updated.

Lightweight Working Agreement
-----------------------------
- Keep WIP ≤ 2 issues.
- Prefer small, reviewable PRs (≤ ~400 LOC diff) for faster iteration.
- Co-locate rationale in PR description for future archeology.

Next Immediate Actions (Recommended Ordering)
---------------------------------------------
1. Implement #1 and #2 together (shared metrics emission touchpoints).
2. Add #3 structured logging (enables richer replay & #15 groundwork).
3. Proceed with #4 tracing spans to baseline scan latency distributions pre-sizing changes.

Maintenance Ritual (Suggested)
------------------------------
- Weekly: Review Backlog ordering; ensure next Ready item aligns with current objectives.
- Bi-weekly: Re-evaluate performance benchmarks (once benches exist) to catch regressions early.
- Monthly: Trim stale ideas / convert ad-hoc notes into issues or discard.

FAQ
----
Q: Why not create real GitHub milestones yet?  
A: Can be added later; current focus is rapid iteration. Logical milestones documented here for clarity without adding process overhead.

Q: When to parallelize work (sharding)?  
A: After sizing logic (M2) is stable to avoid duplicating complexity across shards prematurely.

Revision History
---------------
2025-08-31: Initial scaffold added.
