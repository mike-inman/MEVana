# Local Feature Branch Review — 2026-10-16 17:01 CDT

## Purpose

Identify useful work on older local feature branches that is absent from the reconciled `main`, without replacing current code or enabling trading. Reviewed against `main` at `94be5d2d`.

## Findings

| Branch | Finding | Decision |
| --- | --- | --- |
| `feature/extractor-local-caches` (`40e7d69a`) | Its Kafka `FAST_MODE` consumer and producer settings already exist in `crates/normalizer-service/src/bin/tap_orca_extract.rs`. The branch also changes `crates/ui-web`, which is absent from current `main`. | No extractor code to import. Treat UI changes as historical reference. |
| `feature/whirlpool-core-seeding-and-docs` (`a8e4442a`) | The `whirlpool-core` source, quoter, tick window, and seeding example are present on `main`; the current workspace build passed. The branch also contains older stream and feed components. | Do not replace the current crate. Review any old stream component only against a specific new requirement. |
| `feature/arb-pipeline-next-steps` (`6993a5ae`) | Adds a standalone `arb-test-mode` crate absent from `main`. Its opportunity estimate uses a rough trade-size impact heuristic and logs `WOULD TRADE`; it is not evidence of executable arbitrage profitability. It also modifies the absent `ui-gateway`. | Retain as a research reference. Do not import or deploy as trading logic. |
| `feature/ui-development` (`ead840bb`) and `feature/ui-v2-toggle-pool` (`f098121a`) | Add and extend `crates/ui-web` and `crates/ui-gateway`, neither of which exists in current `main`. Some generated normalizer program files from the later branch are already present. | UI needs a separate product and build review before reintroduction. Do not copy the old UI wholesale. |
| `feat/whirlpool-tx-stream-slim` (`ae9029a8`) | Adds a Dockerfile for the absent `dex-stream-universal` crate and an old environment loader. The Dockerfile's healthcheck URL is malformed (`http://:8080/health`). | Do not import the deployment files as-is. |
| `feature/normalizer-codegen-improvements` (`2df76fd1`) | Tip adds a historical handoff document; its older codegen ancestors have since diverged from current code. | Reference only when a codegen requirement arises. |
| `feature/universal-geyser-stream` (`5abcaf9d`) | Tip adds a historical low-latency deployment plan. | Reference for infrastructure planning; no runtime import. |

## Verification and next slice

- Current `main` passed `dotnet test dotnet/MEVana.slnx --verbosity quiet` (40 tests) and `cargo build --workspace --locked` on 2026-10-16. These checks do not validate the old branch components.
- The reviewed feature branches diverged before the reconciled `main`; a whole-branch merge would reintroduce older or absent components.
- The old UI comparison is recorded below. Use the separate UI project for future UI work and verify its live API contract before changing either repository.
- For arbitrage research, use real input data and measured outcomes before adopting any old opportunity estimator. Keep trading disabled.

## Separate UI comparison — 2026-10-16

The old `feature/ui-v2-toggle-pool` branch contains a Vite application in `crates/ui-web` and a Rust `crates/ui-gateway`. The separate `/Users/michaelinman/Repos/MEVana-UI` repository is a Next.js application with dashboard, swaps, geyser, and wallet pages. It was inspected read-only: its `main` was two commits ahead of `origin/main` and had existing uncommitted changes.

| Old embedded UI | Current location or status | Decision |
| --- | --- | --- |
| Prices and swaps WebSockets at `/ws/prices`, `/ws/swaps`, and `/ws/swaps.v2` | Current `crates/api-service/src/gateway/routes.rs` serves these under `/gateway/ws/`; `MEVana-UI/src/lib/gateway.ts` points to them. | Keep the current gateway and separate UI. Live endpoint behavior remains to be verified. |
| `SwapsPanel` and WebSocket connection status | `MEVana-UI` has a live swaps page and `SwapStream` component. | Do not port the old Vite component. |
| `PoolPanel` | The old panel always sets its pool to `null` and displays a placeholder. Current Rust code has `/gateway/depth/:pool` and an arb-engine `/pool/:id` snapshot route. | Define a real pool data contract only if a pool detail view is requested. |
| Old Rust UI gateway | Its main WebSocket routes are represented in the current API gateway. | Do not reintroduce a second gateway. |

No files were changed in `MEVana-UI`, and no old UI code was imported into MEVana. This comparison is static; it does not establish that the live UI or gateway is deployed or operational.
