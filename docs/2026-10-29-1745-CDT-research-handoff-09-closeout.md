# 2026-10-29 17:45 CDT — Research handoff 09 blocker closeout

Created: 2026-10-29 17:45 CDT

## Outcome

**Stopped at the mandatory preflight: the dev/test Geyser subscription returned UNAUTHENTICATED.** No recorder implementation, decoder repair or capture runs followed. This satisfies the handoff's failed-preflight exit path; it does not establish a functioning capture pipeline.

Worktree: `/Users/michaelinman/Repos/MEVana-worktrees/geyser-capture-20261029`.
Branch: `feat/geyser-capture-20261029`.
Base: `e30ceaea` on main, equal to origin/main after fetch. The committed handoff 08 closeout is present and all five existing specs passed strict validation at entry.

## Preflight evidence

| Field | Observed result |
| --- | --- |
| Connection tier | dev-test |
| Endpoint host only | grpc.va.shyft.to |
| Subscription | One account subscription, owner shared_core::programs::ORCA_WHIRLPOOL, Processed commitment |
| TLS channel ready | 323.577 ms |
| Time to first account | Not observed |
| Account updates / total messages | 0 / 0 |
| Total connection/subscription elapsed | 0.482 seconds |
| Error class | grpc_unauthenticated (gRPC UNAUTHENTICATED) |
| Hard stop | 60-second RPC budget; 65-second outer cleanup limit |
| Observed cluster | Unknown; no account inventory received |

A throwaway Python gRPC subscriber generated bindings from the locally cached yellowstone-grpc-proto 4.2.1 geyser.proto and solana-storage.proto. It used TLS native trust, x-token metadata, the owner constant read from shared-core, and ping replies. It rejected nonpublic DNS resolutions. Endpoint and token were selected separately, checking all process-environment aliases before approved .env aliases. Only authorized Geyser keys in the primary checkout root .env were parsed; the file was not sourced, copied or changed. No backup or unrelated secret files were read.

Execution command: `uv run --with grpcio-tools python /tmp/mevana-handoff09-preflight.py`. This temporary probe is not part of the product and does not persist credentials or market data. The process exited normally after reporting a sanitized result; the gRPC channel and stream were closed. No capture or preflight process remains running.

The authentication status does not distinguish an invalid/expired token, an endpoint/token mismatch, or account entitlement restrictions. Provider details were deliberately not printed. There was no retry with alternate credentials, provider-side mutation or credential rotation. Authentication must be resolved by the operator before resuming the required preflight. This result is not a production-feed benchmark.

## Preserved work

- Narrowed openspec/config.yaml's old prohibition to research contract changes, and added standing read-only capture and credential/data protections.
- Created add-geyser-market-data-capture proposal, design, ten-requirement delta and twelve implementation tasks. Strict validation passes the change and five existing specs (6 passed, 0 failed).
- Recorded the worktree plan under docs with date, time and purpose.
- Marked only the preflight gate task complete, including its failed outcome and required closeout. All eleven downstream tasks remain unchecked. The change stays active and is not archived because implementation is incomplete.

## Evidence unavailable because the gate failed

Neither the five-minute discovery run nor the ten-minute allowlist run occurred. There are no captures, manifests, fixtures, decoded inventories, allowlist, throughput, volume, gap/stall counts, decode rates, peak-memory results or verified cluster. The three requested mainnet mints were not verified against data. Hour/day volume projections, seven/thirty-day disk needs, retention and production-host recommendations cannot be grounded in this run and remain pending.

No Rust, Cargo, decoder, port configuration, .NET, UI, executor, hotpath, Kafka, PostgreSQL or Docker changes were made. No signing, wallet access or transaction construction/submission occurred. No capture data was committed. Credentials and passwords were preserved.

## Follow-ups

Resolve Geyser authentication for the selected endpoint/token pair without disclosing secrets, then rerun the same bounded account preflight before starting task 2.1. Keep the remaining implementation and live-run acceptance gates intact.

Existing findings retained without fixes: JUPITER_V6 contains the historical Jupiter v4 identifier; port 9898 conflicts between token_registry_service configuration and UI executor admin configuration; Redis is 6379 in ports.toml versus 6380 in CLAUDE.md. The known TickArray layout bug remains unfixed because the preflight gate stopped implementation.

## Validation and Git disposition

Only documentation/OpenSpec changes exist, so Rust builds, tests, clippy, decoder tests and live capture acceptance were not run after the failed gate. No application code changed. Final validation and publication results are recorded below before committing this closeout.

- `openspec validate --all --strict`: 6 passed, 0 failed (five existing specs and the proposed change).
- `openspec instructions apply --change add-geyser-market-data-capture --json`: 1 of 12 tasks complete; 11 pending. CLI readiness does not override the handoff authentication gate.
- `dotnet test dotnet/MEVana.slnx --verbosity quiet`: 194 passed, 0 failed, 0 skipped, matching handoff 08.
- `./scripts/ci/repo_hygiene.sh`: passed, with existing timestamp/location warning categories; no unrelated cleanup performed.
- `git diff --check` and staged diff checks: passed.
- Non-printing approved-key full-value/long-prefix scan: zero matches in changed or untracked content. Repository-wide git grep also found pre-existing matches; no existing credentials or related files were removed or altered. No blanket repository secret-cleanliness claim is made.
- Final pre-publication `git fetch origin` and `git rev-list --left-right --count main...origin/main`: 0 / 0. No remote-only main commits require merging. The isolated branch descends from synchronized main; publication targets this branch without modifying main or other worktrees.
- Logical commits: OpenSpec context/proposal/design/tasks and dated plan, then this blocker closeout. Push command: `git push -u origin feat/geyser-capture-20261029` (normal fast-forward branch publication, never force). Verify upstream equality and a clean worktree after publication.
