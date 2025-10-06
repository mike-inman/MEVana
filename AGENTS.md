# Repository Guidelines
Last updated: 2025-10-09

## Project Structure & Module Organization
MEVana is a multi-crate Rust workspace; primary services live under `crates/` (e.g., `hotpath_arb_engine`, `executor-service`, `normalizer-service`). Shared libraries reside in `crates/shared-*`, while generated normalizer artifacts sit in `crates/normalizer-generated`. Keep Docker Compose, Prometheus, and Grafana assets inside `infrastructure/` (dashboards under `infrastructure/grafana/dashboards/`, datasources in `infrastructure/grafana/datasources/`). Place operational scripts inside `scripts/` using the existing subfolders (`deployment/`, `maintenance/`, `monitoring/`, `testing/`). Documentation belongs in `docs/` and should reuse the established subdirectories.

## Build, Test, and Development Commands
Run `cargo build --workspace` for a full workspace build and `cargo run -p hotpath_arb_engine --bin hotpath_arb_engine` to exercise the hotpath locally (export env overrides from `config/environments/*.env.sample`). Use `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` before every PR; these match CI. Execute `cargo test --workspace` for unit and integration coverage, and `bash scripts/ci/repo_hygiene.sh` to mirror the CI hygiene gate. Spin up the supporting stack with `docker compose -f infrastructure/docker-compose.core.yml up -d --build` when you need Kafka/Redis/Postgres.

## Coding Style & Naming Conventions
Follow `docs/development/RUST_STYLE.md`: types use CamelCase, functions and modules use snake_case, constants are SCREAMING_SNAKE_CASE. Keep `main.rs` thin, prefer structured `tracing` logs, and avoid `unwrap()` in production paths. Stick to four-space indentation, group imports explicitly, and document public APIs with rustdoc examples. Justify any lint suppressions and scope them narrowly.

## Testing Guidelines
Co-locate unit tests in each crate and add cross-crate scenarios under `tests/`. Heavy integration suites should be marked with `#[ignore]` and documented with run instructions. For Kafka or infrastructure-dependent checks, leverage helpers in `scripts/testing/` or the `scripts/deployment/bootstrap_kafka.sh` workflow. Confirm coverage for new metrics or endpoints by adding assertions or snapshot tests where feasible.

## Commit & Pull Request Guidelines
Write commits in a concise imperative voice (`docs: add rebuild plan`, `Integrate edge delta pipeline`). Squash unrelated work, explain directory placement when adding files, and mention any follow-up tasks. Pull requests should include a summary, test evidence (`cargo fmt`, `cargo clippy`, `cargo test`), and links to issues or runbooks touched. Attach screenshots for UI-facing changes and note deployment or rollback considerations when relevant.
