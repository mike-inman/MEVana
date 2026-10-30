# 2026-10-30 01:55 CDT — Handoff 11 exact-quote blocker

## Disposition

Handoff 11 remains incomplete. The requested all-pool, same-pair replay cannot yet produce a truthful `Simulated` result from the allowlist session. The exact quoter did not return for a captured SOL/USDT route at slot `463131598`, even after reducing a diagnostic input to 0.1 USDT. No candidate was assigned an invented loss, win or `insufficient_state` reason. No default result, sensitivity result or determinism hash exists.

## Reproduction and evidence

- Input: the four finalized files in `~/MEVanaData/captures/handoff09-20261029/capture-1793330490539629000/`. The new loader verified each SHA-256 against its manifest, required the expected session identity, and read only local files.
- The replay reached the SOL/USDT ordered route `B6LL9aCWVuo1tTcJoYvCTDqYrq1vjMfci8uHxsm4UxTR` → `BmXVhzBHTawt5stWKBtFdjM8n33G5RoUDECE5FrRvayq` at slot `463131598` after applying that slot's writes.
- With 10,000,000 USDT atoms (10 USDT) at the route start, the first exact leg produced 104,228,391 SOL lamports. The next exact leg entered `BmXV...` with captured pool liquidity `120,417,502` and did not return during the bounded diagnostic period.
- With a diagnostic ladder starting at 100,000 USDT atoms (0.1 USDT), the same route's first leg produced 1,044,034 SOL lamports. The `BmXV...` exact leg again did not return after more than 60 seconds at full CPU. A one-second process stack sample was entirely under `research_replay::quote_leg` and `orca_whirlpools_core::quote::swap::compute_swap_step`.
- The 1,000-unit exploratory size had also shown prohibitively long exact evaluation on a different SOL/USDC route. The smaller diagnostic ladders were **not** accepted as experiment defaults. Temporary debug logging and ladder changes were removed from source after diagnosis.

This is evidence of an unbounded or prohibitively slow quote in this captured state, not proof of an infinite loop or a price/math conclusion. The source of the problem might be a no-progress iteration in the pinned Orca core, inconsistent Processed account state, or an unseen directional array. We have not isolated which. The current `whirlpool_core::exact` API has no bounded-step result, and this handoff prohibits changing its quote behavior. Classifying the route as a missing array or excluding the pool without evidence would bend the result.

## Saved work

- Branch `feat/deterministic-replay-20261030` was rebased onto `origin/main` at `43c48a40`; the original contract-blocker report is retained as history.
- The opt-in replay crate, manifest verification, slot ordering/state/gap logic, route generation, exact-quote path, policies, accounting and output format are committed. The OpenSpec change `add-deterministic-replay` is active and validates, with tasks still open.
- Draft test-only capture catalog and C# validation tests are preserved, but their three pair inputs are in one explicitly skipped xUnit theory until completed replay artifacts exist. All 194 existing .NET tests passed; no contract behavior changed.
- No provider call, capture mutation, signing, transaction or trade occurred. Diagnostic processes were stopped; no replay output directory was created.

## Validation at the stop boundary

The checks below used `~/.cargo/bin` first on `PATH`, so both cargo and rustc were `1.92.0-nightly`. They validate saved code and existing contracts; they do not validate a completed replay result.

| Check | Result |
| --- | --- |
| `cargo fmt -p research-replay -- --check` | Passed |
| `cargo clippy --offline --locked -p research-replay --all-targets -- -D warnings` | Passed |
| `cargo test --offline --locked -p research-replay -p whirlpool-core -p geyser-capture` | Passed |
| `dotnet test dotnet/MEVana.slnx --verbosity quiet` | 194 passed, 1 skipped fixture theory, 0 failed |
| `openspec validate --all --strict` | 8 items passed |
| `./scripts/ci/repo_hygiene.sh` | Passed with existing documentation and script-location warnings |
| `git diff --check` | Passed |
| Non-printing credential-pattern scan over changed content | No matches |

The OpenSpec change remains active because its replay-evidence tasks are incomplete. The archive workflow requires a completed task set or an explicit decision to archive incomplete work; this report does not present the incomplete slice as completed.

## Required next decision

Investigate the captured `BmXV...` pool/tick-array state and the pinned exact quote's progress behavior in a separately authorized quote-safety slice. A deterministic, integer-only bounded quote failure or evidence-backed missing-state classification is needed before replaying all five SOL/USDT pools. Then rerun all three pair defaults, pair A sensitivities, C# validation and determinism checks. Do not archive this OpenSpec change or merge the branch until those gates pass.

The cost-reference ambiguity also remains: pair A requires all six SOL/USDC pools in routes, while Revision 1 says the SOL/USDC reference pool is never traded. The current design treats its cost quote as a separate price input but allows that same captured pool in routes. Resolve that wording before final acceptance.
