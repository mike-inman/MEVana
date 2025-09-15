AI Assistants Guidelines (MEVana)

Your outputs must follow these rules to keep the repo consistent:

Do
- Place Grafana dashboards only in `infrastructure/grafana/dashboards/`.
- Place Grafana datasources only in `infrastructure/grafana/datasources/`.
- Put all Docker Compose and monitoring config under `infrastructure/`.
- Put operational scripts in `scripts/` (subfolders: `deployment/`, `maintenance/`, `monitoring/`, `testing/`).
- Put documentation only in `docs/` (and organize under `docs/operations/`, `docs/deployment/`, `docs/architecture/`, `docs/strategy/`, `docs/reference/`).
- Use path-agnostic shell: prefer `tar -C`, `repo_root=$(git rev-parse --show-toplevel)`.

Rust code requirements
- Follow `docs/development/RUST_STYLE.md` for naming, error handling, logging, and async patterns.
- Format and lint before proposing patches: `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.

Documentation timestamps
- When creating or updating any Markdown under `docs/**`, include a prominent timestamp near the top:
  - `Last updated: YYYY-MM-DD` for modifications
  - `Created: YYYY-MM-DD` for new docs

Don’t
- Don’t create new top-level directories without an approved issue/ADR.
- Don’t put dashboards in `dashboards/` at the repo root.
- Don’t commit archives (`*.tgz`, `*.zip`), OS files (`.DS_Store`, `._*`), or secrets.
- Don’t fork documentation structures (no `docs-new/`).

PR hygiene
- Explain directory choices and confirm they match these rules.
- Keep changes minimal and focused; avoid unrelated edits.
