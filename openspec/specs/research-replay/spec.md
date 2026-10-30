# research-replay Specification

## Purpose

Offline research replay reconstructs captured market state and produces reproducible, scoped simulated comparisons of route selection policies without trading authority.

## Requirements

### Requirement: Verified offline input
Replay SHALL accept finalized allowlist captures of format v1 or v2, verify each file against its manifest SHA-256, and record input hashes and configuration in an output manifest. It SHALL perform no provider, RPC, signing or transaction operation.

#### Scenario: Input identity
- **WHEN** a finalized allowlist capture is replayed twice with identical configuration
- **THEN** each output file has identical bytes and hashes

#### Scenario: Corrupt input
- **WHEN** a capture file differs from its manifest, contains an unknown frame kind or is incomplete
- **THEN** replay rejects the input without producing a completed result

### Requirement: Event-time state
Replay SHALL reconstruct slot parent/status history, buffer account writes, and by default apply only confirmed-fork writes at each slot's first Confirmed event. Evaluation and latency SHALL use confirmed-slot order. Processed mode SHALL remain available. Skipped leader slots SHALL not mark state stale; true gaps SHALL. Unseen pool or tick-array state SHALL be insufficient state, never assumed empty.

#### Scenario: Gap
- **WHEN** a non-discontinuity capture gap, an unseen parent, or an unexplained discontinuity occurs
- **THEN** candidates touching stale pools receive `state_gap` until a covering snapshot or later pool write restores state

#### Scenario: Missing array
- **WHEN** an exact quote needs an unseen tick array
- **THEN** the candidate receives `insufficient_state`, unless a current snapshot proves the array absent, in which case it receives `tick_array_absent`

#### Scenario: Skipped slot
- **WHEN** a slot jump has a known parent link
- **THEN** the jump does not create a state gap

#### Scenario: Abandoned fork
- **WHEN** writes belong to slots never Confirmed in the capture
- **THEN** confirmed replay drops and counts those writes, and they never alter state

### Requirement: Scoped same-pair routes
Replay SHALL evaluate ordered distinct-pool two-leg routes for each standard-fee pair separately: six SOL/USDC pools, five SOL/USDT pools and two USDC/USDT pools. Each pair SHALL have a canonical v1 request naming exactly its evaluated pools; its second token SHALL be the quote asset. Token continuity and mint-derived direction SHALL be checked at every hop. Adaptive-fee pools and triangles SHALL not be evaluated.

#### Scenario: Three pair results
- **WHEN** the allowlist replay completes
- **THEN** it produces separate Simulated results with 30, 20 and 2 route shapes respectively and each request/result validates against the unchanged contract

### Requirement: Exact integer economics
Every route size and SOL-to-quote cost conversion SHALL use the exact Whirlpool quoter. Costs SHALL include base, priority and tip lamports and be subtracted in quote atoms using checked integer arithmetic. Estimated net at detection slot and simulated net after declared slot latency SHALL remain separate; missing future state SHALL not become a simulated loss or win.

#### Scenario: Future quote
- **WHEN** a selected route is requoted at slot S plus latency
- **THEN** simulated net comes from that later state, while estimated net remains from S

### Requirement: Fair policy comparison
FixedUniverse, SimpleRules and the explicitly heuristic `ranking-v0` ModelRanking SHALL see the same candidate universe and cost model. Each candidate SHALL belong to exactly one selected or rejected bucket per policy. Counts, signed estimated/simulated profit totals and reason-coded rejections SHALL be emitted. No ML model or fill competition SHALL be implied.

#### Scenario: Accounting
- **WHEN** a pair result is completed
- **THEN** each policy's selected count plus rejection counts equals its evaluated count, with missed null and realized profit null

### Requirement: Reproducible artifacts
Replay SHALL write a lossless per-candidate JSONL, summaries, three canonical requests and results, and a run manifest outside Git. Only bounded request/result fixtures and a sample of at most 64 KB SHALL be committed.

#### Scenario: Repeat
- **WHEN** identical capture, seed and parameters are replayed
- **THEN** all generated artifacts are byte-identical
