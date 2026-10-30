# Spec Delta — 2026-10-30 CDT

## MODIFIED Requirements

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

### Requirement: Verified offline input
Replay SHALL accept finalized allowlist captures of format v1 or v2, verify each file against its manifest SHA-256, and record input hashes and configuration in an output manifest. It SHALL perform no provider, RPC, signing or transaction operation.

#### Scenario: Input identity
- **WHEN** a finalized allowlist capture is replayed twice with identical configuration
- **THEN** each output file has identical bytes and hashes

#### Scenario: Corrupt input
- **WHEN** a capture file differs from its manifest, contains an unknown frame kind or is incomplete
- **THEN** replay rejects the input without producing a completed result
