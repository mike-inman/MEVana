# Spec Delta — 2026-10-30 CDT

## ADDED Requirements

### Requirement: Opt-in bounded snapshot
With `--snapshot-rpc tatum`, capture SHALL use only the fixed HTTPS Tatum endpoint and its TATUM_MAINNET credential for confirmed pool and nearby tick-array snapshots. It SHALL retain unchanged behavior when the flag is off, and SHALL enforce per-call, per-snapshot and session budgets.

#### Scenario: Bounded source
- **WHEN** snapshot mode is enabled after the first Confirmed slot or a true reconnect gap
- **THEN** at most 6 calls and 25 MB total response bytes are used per snapshot, at most 5 snapshots occur, and each call has a 10-second timeout with at most 2 retries except on 401/403

#### Scenario: Credential isolation
- **WHEN** a snapshot RPC request is made
- **THEN** only TATUM_MAINNET is sent as `x-api-key` to `https://solana-mainnet.gateway.tatum.io/`, and no credential is serialized or logged

### Requirement: Snapshot format v2
Snapshot-enabled sessions SHALL use formatVersion 2 and frame kind 2 for versioned snapshot records, including context slot, key, presence, and, for present accounts, owner, lamports, data and SHA-256. Inspection SHALL validate these frames and manifests SHALL count snapshot calls, bytes, present and absent accounts. Existing v1 captures SHALL remain inspectable.

#### Scenario: Absent array
- **WHEN** a requested tick-array PDA is absent at the confirmed snapshot context
- **THEN** the snapshot records its key and absence without fabricating account data

#### Scenario: Snapshot failure
- **WHEN** a bounded snapshot fails
- **THEN** a secret-free failure frame is written and streaming continues under no-snapshot replay rules
