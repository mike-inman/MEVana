# Design — 2026-10-30 CDT

## Context

See `proposal.md` and the handoff 12 document. Handoff 11's exact integer quoter and v1 request/result contract remain unchanged. The old capture has slot parent/status frames but no initial tick-array snapshot. Provider use is forbidden until offline Part A passes the specified reconciliation.

## Goals / Non-Goals

**Goals:** Correct event-time replay, bounded read-only snapshot capture, versioned format compatibility, deterministic per-pair Simulated results.

**Non-Goals:** Trading, wallet access, triangles, adaptive-fee quoting, learned ranking, server deployment or changed C# contract behavior.

## Decisions

1. Parse connection-0 slot frames into a slot graph keyed by slot, keeping parent and first Processed/Confirmed/Finalized/Dead receive timestamp and sequence. Use receive timestamp and sequence for confirmation order. A confirmed slot's buffered writes are applied in `(slot, write_version, sequence)` order at first confirmation. Keep processed mode separately; use confirmation-order indices for latency rather than arithmetic slot IDs. Writes in never-confirmed slots are dropped and counted by slot.
2. Classify discontinuity gaps with the graph. A parent seen in the capture explains a jump; skipped leader slots do not stale state. Non-discontinuity account gaps and missing parents are true gaps. Manifest records every classification. A true gap stales pools until a covering snapshot or fresh pool write. This avoids guessing that missing numbers indicate lost market data.
3. Keep handoff 11 artifacts immutable. Version new results `research-replay-0_2_0`, use new result IDs and test fixtures. Run old capture defaults and A processed sensitivity before any network use. Stop if not seven skipped, one fork, zero true gaps and fork writes dropped.
4. Add opt-in `--snapshot-rpc tatum` to capture. Start the TLS Geyser stream first, snapshot after first Confirmed slot and after true account reconnects only. Use a fixed HTTPS endpoint, a dedicated Tatum key read with existing environment-before-root-.env precedence, and an HTTP client configured for rustls without OpenSSL. Fetch all 18 pool accounts, derive 13 tick-array PDAs around each current tick, and fetch keys in batches of at most 100. Record confirmed context slots and verify Whirlpool ownership and data hashes.
5. Version snapshot records as JSON in frame kind 2, retaining the existing frame envelope. FormatVersion 2 is used only when snapshot mode is enabled; v1 remains unchanged. Hard counters reject any attempt to exceed six calls/snapshot, five snapshots/session or 25 MB response bytes/session; calls timeout after 10 seconds and retry at most twice, except no 401/403 retry. Failure writes a redacted `snapshot_failed` frame and streaming continues. Replay ignores stream writes at or before an account's snapshot context and distinguishes proven absent arrays from unseen arrays.
6. Use only one new capture of at most 600 seconds under the handoff 12 data directory. Re-run default pairs twice, plus pair A latency, cost and processed sensitivities. Generate new test-only catalog entries/fixtures and validate the unchanged v1 contract.

## Risks / Trade-offs

- Snapshot context slots can differ across batches → retain each batch's context and compare stream write slots per account.
- Snapshot failures or incomplete arrays → retain `insufficient_state`; never assume empty state.
- Processed dev/test feed and laptop timing → report evidence as Simulated only.
- Provider quota or TLS failure → stop after bounded preflight/failure, leave no open process/socket.

## Migration Plan

Part A is entirely offline and must pass before Part B. Format v1 remains readable; format v2 is opt-in. No service migration. Merging remains the operator's decision.
