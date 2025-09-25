# Repository Guidelines
Created: 2025-09-25

## Project Structure & Module Organization
MEVana is a Rust workspace rooted in `Cargo.toml`; each feature lives in `crates/<crate-name>/` with shared types in `crates/shared-core` and `crates/shared-protocol`. Infrastructure and operational manifests belong in `infrastructure/` (Docker Compose, Prometheus, Grafana), while runtime scripts live under `scripts/` with domain-specific subfolders such as `scripts/deployment/` and `scripts/monitoring/`. Documentation stays within `docs/**`—architecture notes in `docs/architecture/`, operations guides in `docs/operations/`, and reference material like this guide in `docs/reference/`. Test fixtures and data helpers sit alongside their crates, typically under `crates/*/tests` or `crates/*/fixtures`.

## Build, Test, and Development Commands
Use `cargo build --workspace` for a full compile and `cargo test --workspace` before opening a PR. Lint and formatting gates mirror CI: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`. Run `bash scripts/ci/repo_hygiene.sh` to execute the same hygiene checks used by automation. For a local stack, `./scripts/start_system.sh` brings up required services, or use `docker compose -f infrastructure/docker-compose.core.yml up -d --build` when testing container overlays.

## Coding Style & Naming Conventions
Follow `docs/development/RUST_STYLE.md`: prefer 4-space indentation, `snake_case` modules and functions, and `UpperCamelCase` types. Surface errors with `anyhow::Result` at binary edges and `thiserror` for library enums. Log via `tracing` macros with structured key-value fields (`tracing::info!(order_id, ...)`). Async code should leverage `tokio` primitives and propagate cancellation cleanly.

## Testing Guidelines
Unit and integration tests live beside their crates; use module-level `#[cfg(test)]` blocks for small units and integration harnesses in `crates/<crate>/tests`. Favor descriptive names like `test_handles_partial_fill` and ensure fixtures mimic Solana on-chain data. Run targeted suites with `cargo test -p <crate-name>` when iterating, and execute the full workspace test suite plus `cargo deny check` before merging significant changes.

## Commit & Pull Request Guidelines
Commit messages follow Conventional Commits (`feat:`, `fix:`, `docs:`) as seen in recent history. Keep commits scoped, mention affected crates or infra paths, and avoid bundling unrelated changes. PRs should complete the template: explain directory placement, confirm no archives or secrets were added, list tests executed, and link issues or deployment notes when relevant. Include screenshots or logs if the change impacts dashboards or monitoring output.
