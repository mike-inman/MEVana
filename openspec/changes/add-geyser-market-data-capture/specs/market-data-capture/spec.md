# 2026-10-29 — Market-data capture

## Purpose

Preserve public Geyser market-data bytes with explicit integrity, gaps and source provenance for later quoting and replay research.

## ADDED Requirements

### Requirement: Read-only source and credentials
Capture SHALL record public market data without signing, wallet access, transaction construction or submission. Credentials SHALL use process environment before approved root .env Geyser keys, and SHALL never be serialized, logged or included in errors.

#### Scenario: Credential selection
- **WHEN** both environment and .env contain authorized aliases
- **THEN** exported values win across endpoint aliases GEYSER_ENDPOINT, SHYFT_GEYSER_URL, ENDPOINT and token aliases GEYSER_X_TOKEN, X_TOKEN, SHYFT_GEYSER_X_TOKEN; unrelated keys are never loaded

### Requirement: Source classification
Capture SHALL reject private, loopback and localhost sources unless explicitly synthetic. Every manifest SHALL carry source classification, configurable connection tier defaulting to dev-test, host only and observed cluster.

#### Scenario: Synthetic endpoint
- **WHEN** a local or privately resolved endpoint is requested
- **THEN** startup fails unless --synthetic-source is set; with it all outputs are marked Synthetic and excluded from market catalogs

### Requirement: Account and transaction subscriptions
Capture SHALL subscribe at configurable commitment (Processed default) to slots, all Whirlpool pools by IDL-derived size and allowlisted TickArrays by IDL-derived size and pool offset. It SHALL exclude positions and optionally include non-vote pool transactions, including failures, enabled by default.

#### Scenario: Empty and populated allowlists
- **WHEN** discovery has an empty allowlist or targeted capture has selected pools
- **THEN** discovery requests pools and slots only; targeted filters select the union of allowlisted pools using separate memcmp alternatives rather than an impossible AND of pool keys

### Requirement: Gap visibility
Capture SHALL answer pings, record every disconnect, reconnect, stall and slot discontinuity, and retry with jittered capped exponential backoff no greater than 30 seconds. Stall timeout SHALL be configurable.

#### Scenario: Interrupted mock source
- **WHEN** updates stall, disconnect or skip a slot
- **THEN** explicit timestamped gap records survive rotation and inspection and retry delays remain bounded

### Requirement: Raw format v1
Files SHALL contain length-delimited frames with globally increasing sequence, UTC nanosecond receive time and unmodified encoded SubscribeUpdate bytes. Ingest SHALL preserve bytes without decoding or filtering. Files SHALL rotate by size or time via fsync and atomic rename from .part.

#### Scenario: Rotation and interruption
- **WHEN** a limit is reached or capture terminates
- **THEN** completed files are durable with manifests; crashes retain .part files reported as incomplete and never cataloged

### Requirement: File and session provenance
Each finalized file SHALL have SHA-256, bytes/frames, slot and UTC coverage, commitment, filters/allowlist, endpoint host, connection tier, source/cluster, crate version, git commit and gap counts/details. A session manifest SHALL list file manifests.

#### Scenario: Dataset mapping
- **WHEN** a valid capture is considered for DatasetMetadata
- **THEN** session identity maps to Id, format version to MetadataVersion, coverage to UTC bounds and sanitized capture identity to Provenance without slash, backslash or ://; tier is retained and catalog completeness is separately validated

### Requirement: Bounded operation and metrics
Capture SHALL accept output directory, rotation limits, commitment, allowlist, transactions, stall timeout, connection tier and maximum duration. Default output SHALL be ~/MEVanaData/captures outside the repo. It SHALL bind configurable metrics at 127.0.0.1:8121 using geyser_capture_ names and fail if occupied.

#### Scenario: Hard stop and shutdown
- **WHEN** duration expires or SIGINT/SIGTERM arrives
- **THEN** capture finalizes current data and manifests, closes all resources and exits; default rotation is hourly or configured size cap

### Requirement: Integrity and decoding inspection
capture_inspect SHALL verify lengths, sequence and manifest hashes, report .part files and gaps, summarize decode success/failure by type and produce pool inventory with address, mints, tick spacing, fee rate and update count. Unexpected account types or sizes SHALL be reported.

#### Scenario: Corrupt and valid inputs
- **WHEN** inspection sees corruption, incomplete files or real pool/TickArray records
- **THEN** corruption fails integrity validation, incomplete data is excluded, and successful decodes appear in inventory with explicit failure counts

### Requirement: IDL layout and offline evidence
Filters and decoder lengths SHALL agree with the repository IDL. Offline tests SHALL cover round trips, rotation, incomplete files, hashes, gaps with a mock source, backoff bounds, offsets, sizes and secret redaction. Real public fixtures SHALL include pool/slot identity and a plausible TickArray.

#### Scenario: Wrong TickArray length
- **WHEN** a TickArray has an unexpected length
- **THEN** decoding fails instead of silently accepting a mismatched layout

### Requirement: Live validation boundary
A failed connection/authentication preflight SHALL halt development with a blocker closeout. Successful preflight SHALL precede five-minute discovery and a bounded ten-minute allowlist run with 2–3 minute rotation and shutdown validation. No live run SHALL exceed ten minutes or remain running.

#### Scenario: Dev/test reporting
- **WHEN** short-run evidence is reported
- **THEN** report rates by type, bytes, gaps/stalls, decode rates and peak memory; qualify extrapolations and cluster, commit only small decoder fixtures and observed SOL/USDC, SOL/USDT, USDC/USDT allowlist, and make no profitability or production-completeness claims
