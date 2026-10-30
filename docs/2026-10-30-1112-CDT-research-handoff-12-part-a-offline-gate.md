# 2026-10-30 11:12 CDT — Handoff 12 Part A offline gate

Part A was completed **before any network or provider use**. The pinned rustup Cargo was `1.92.0-nightly`. The handoff 09 allowlist capture was read from local disk only. These trial outputs are outside Git under `~/MEVanaData/replay/handoff12-partA-trial2/` and `handoff12-partA-trial3/`; every generated file matched byte-for-byte. The trial manifests label their code argument `partA-trial2`; the final closeout will distinguish them from commit-pinned outputs.

The connection-0 slot graph yielded **seven skipped-leader discontinuities, one abandoned-fork discontinuity and zero true gaps** within the replay window. A parent before the first evaluated slot was excluded from internal gap classification, since it lies outside the captured replay window. Confirmed mode dropped account writes from 463132684 (1), 463132685 (1) and 463132686 (6). It also dropped 463132687 (4) and late unconfirmed tail writes at 463134134 (1) and 463134135 (4), all listed in the manifest. No dropped write reached confirmed state. A synthetic test covers skipped, fork, orphan and reconnect cases, and a separate synthetic test verifies fork writes cannot mutate confirmed state.

| Pair | Handoff 11 state gap | Part A state gap | Handoff 11 insufficient | Part A insufficient | Handoff 11 evaluable | Part A evaluable |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A SOL/USDC | 26.75% | 0% | 5.20% | 5.74% | 68.05% | 94.26% |
| B SOL/USDT | 56.16% | 0% | 29.26% | 72.41% | 14.58% | 27.59% |
| C USDC/USDT | 58.01% | 0% | 29.73% | 40.16% | 12.25% | 59.84% |

Confirmed replay evaluated 2,647 boundaries: A 79,410, B 52,940 and C 5,294 policy candidates. Every policy in each pair detected and selected zero opportunities, so estimated and simulated totals were zero. Pair A's processed sensitivity evaluated 2,651 boundaries and 79,530 candidates per policy; it had zero state gaps, 4,560 insufficient-state and 74,970 below-threshold candidates, with zero detections/selections. The rise in B/C `insufficient_state` after removing false gaps makes snapshot coverage important; it is not evidence of profitable or unprofitable execution.

Offline validation passed: `cargo fmt -p research-replay -- --check`; locked offline Clippy with `-D warnings`; locked offline tests for research-replay, geyser-capture and whirlpool-core; 197/197 .NET tests; OpenSpec strict validation; repo hygiene (existing warnings); `git diff --check`; and a non-printing credential-pattern scan over the diff. No Geyser/RPC/provider call was made. This satisfies the Part A gate for beginning Part B implementation and its one bounded capture.
