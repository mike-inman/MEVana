# 2026-10-29 — Geyser capture design

## Context

See proposal.md. The IDL, not existing incorrect TickArray types, is authoritative. Existing Geyser client patterns supply TLS, owner filtering and keepalive behavior but no recorder. Handoff 08 is complete and entry main equals origin/main at e30ceaea.

## Goals / Non-Goals

Preserve raw protobuf wire messages and bounded memory while maintaining explicit gaps. No quote, replay, catalog writer, trading, infrastructure or existing-service changes.

## Decisions

- Run the required throwaway preflight first. Generate Python protobuf bindings from the locally cached Yellowstone 4.2.1 proto for a single owner-filtered stream. Do not start recorder development if connection/authentication fails. All diagnostics use fixed classes, never raw provider error details.
- Build the Rust recorder only after success, with locked dependencies. Use a raw-byte gRPC codec so unknown fields and encoding survive; decoding for inventory/coverage occurs after durable ingestion, rather than re-encoding Prost messages.
- Use a versioned file header and little-endian length-delimited typed records. Update records contain sequence u64, receive UTC nanoseconds u64 and raw protobuf bytes; gap records contain sequence/time and structured reason/details. Cap frame lengths during inspection. Use monotonically allocated file names with exclusive creation, incremental SHA-256 and counters; fsync data then rename, atomically persist manifest and fsync the directory. Session manifests list only finalized file manifests. Explicit empty sessions are valid but never claimed as available datasets.
- Derive account layouts recursively from idls/whirlpool_idl.json and cross-check serialized corrected Rust layouts: Tick 113, TickArray 9988, pool key offset 9956. Pool size is computed independently from the IDL. Separate named TickArray filters implement OR across allowlist pools. Empty allowlist suppresses tick and transaction filters.
- Keep credentials in a non-serializable type with redacted Debug. Load only approved keys, preferring any process-environment alias over .env aliases. Inspect parsed hosts and resolved addresses; reject private sources unless Synthetic. No endpoint query string is persisted.
- Expose fixed-port loopback metrics with a new prefix; test existing PortConfig parsing without modifying shared-core. Record port 8121 only after conflict/config checks.
- Map stable sanitized session ID to DatasetMetadata.Id and format-v1 to MetadataVersion. Provenance is a slash-free capture-session identifier with tier; coverage is first/last receive UTC. Endpoint stays in capture manifests. Dataset conversion remains later work and must enforce token/pool minimums and all catalog rules.
- Store bulk capture outside checkout. A compressed real TickArray fixture can satisfy the few-KB fixture budget while retaining full public bytes for decoder tests; compression is permitted only with an already locked crate.

## Risks / Trade-offs

- Dev/test restrictions or network limits → stop on failed preflight; report resets/filter limits honestly after success.
- Account layout variants → report unexpected discriminator/size, never force a known decoder.
- Processed slot notifications can repeat or arrive out of order → track slot progression/status separately and avoid treating status repeats as lost slots.
- File and manifest crash windows → .part remains incomplete; missing/mismatched manifests fail inspection and cannot enter catalog.
- Memory/disk pressure → bounded frame size, streaming writes, explicit rotation and measured short-run projections.

## Migration Plan

No deployment. Add isolated crate and restricted decoder fix; run offline and bounded live checks; archive only on successful completion. Preserve branch and pending tasks if the preflight fails. Existing Jupiter ID, 9898 collision and Redis configuration mismatch remain documented follow-ups.
