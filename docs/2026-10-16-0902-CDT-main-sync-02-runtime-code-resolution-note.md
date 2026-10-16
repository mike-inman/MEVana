# 2026-10-16 09:02 CDT — Main sync handoff 02 runtime/code resolution note

## Scope and decision

The 16 assigned runtime paths were compared between local `main` and GitHub `main`. Each path has the same non-string source structure on both sides; together they differ by 27 single-line quoted-literal replacements. These are alternative defaults, examples, messages, or test values rather than complementary blocks of code. Each reviewed final path on `integration/main-sync-20261015` uses the active local `main` version. This follows the owner's instruction to preserve local values while deferring rotation. The last path commits on the two branches have matching timestamps, so timestamps alone cannot establish which literal was created later. No value is reproduced here.

Choosing one branch's literal can change fallback connection settings, token defaults, test expectations, log wording, or script behavior when an explicit environment value is absent. These are not claimed to be equivalent in every environment. No executable branch, function, or statement outside those literals changed in this slice. The final merge must select these reviewed integration versions for the 16 add/add conflicts.

| Assigned path | Differing literal lines | Review result |
| --- | ---: | --- |
| `crates/_archive/api-service-v2-original/src/main.rs` | 2 | Local log/default literals selected; archived runtime retained. |
| `crates/api-service/src/api.rs` | 1 | Local fallback literal selected. |
| `crates/dex-registry/src/config.rs` | 1 | Local configuration default selected. |
| `crates/executor-service/src/bin/replay_export.rs` | 1 | Local CLI default selected. |
| `crates/executor-service/src/execution_engine.rs` | 1 | Local fallback literal selected. |
| `crates/executor-service/src/main.rs` | 1 | Local fallback literal selected. |
| `crates/executor-service/tests/mtls_reload_binary.rs` | 2 | Local test literals selected; the existing PEM test material was not changed. |
| `crates/hotpath_arb_engine/src/main.rs` | 3 | Local CLI, test-token, and fallback literals selected. |
| `crates/hotpath_arb_engine/src/system_health_monitor.rs` | 1 | Local expected test value selected. |
| `crates/kafka_db_writer/src/main.rs` | 1 | Local fallback literal selected. |
| `crates/monitoring-service/src/main.rs` | 1 | Local fallback literal selected. |
| `crates/shared-core/src/database_config.rs` | 6 | Local database URL and password test literals selected. |
| `crates/token-registry-service/src/main.rs` | 1 | Local fallback literal selected. |
| `crates/wallet-vault/src/main.rs` | 1 | Local log-redaction literal selected. |
| `test_hotpath.sh` | 3 | Local test environment defaults selected. |
| `tools/quick_api_server.py` | 1 | Local quick-server mapping literal selected. |

## Verification

- The integration worktree versions matched GitHub `main` before the edits. A line-level comparison confirmed that each changed hunk was one quoted-literal line on each side; 16 files and 27 lines were selected from local `main`.
- `git diff --check` and `git diff --cached --check`: passed.
- Gitleaks 8.30.1 staged scan with full redaction: zero findings in the selected changes. This does not replace handoff 01's history publication gate.
- `bash -n test_hotpath.sh`: passed.
- `python3 -m py_compile tools/quick_api_server.py`: passed.
- `cargo check --locked --offline` across the nine touched Rust packages: blocked by `ethnum` 1.5.2 compile error `E0512` on Rust 1.98.1; the command was stopped after the error while other compilation jobs were still running. This is not a passing cross-crate build.
- `cargo test --locked --offline -p shared-core database_config --lib -- --nocapture`: failed during test compilation because `#[tokio::test]` is unavailable in `crates/shared-core/src/wallet_safety.rs`. The database configuration tests did not run.
- `cargo check --locked --offline -p shared-core --lib`: passed.
- `Cargo.lock`, `crates/shared-core/Cargo.toml`, and `crates/shared-core/src/wallet_safety.rs` are byte-identical between the two `main` tips. The two build blockers therefore are not introduced by the 27 selected runtime literal lines. Their repair belongs to the later properly building baseline.

## Boundary

This is local path-level preparation only. Handoff 01 remains `HOLD` for publishing local-only history. No history merge, GitHub push, credential rotation, live provider request, or deployment is part of this resolution.
