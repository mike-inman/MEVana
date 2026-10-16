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
- The highest-value separate evaluation is the old UI as a product decision. If a UI is desired in this repository, compare it with the current UI project and define the API contract before porting code.
- For arbitrage research, use real input data and measured outcomes before adopting any old opportunity estimator. Keep trading disabled.
