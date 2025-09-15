Run This First — Agent Pre-flight

Last updated: 2025-09-15

Required reading
- AGENTS.md
- docs/ai/ONBOARDING_AI.md
- docs/DOCUMENTATION_INDEX.md
- docs/operations/CRITICAL_PRODUCTION_BLOCKERS.md
- docs/development/RUST_STYLE.md

Local commands
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/ci/repo_hygiene.sh
```

Notes
- Add a timestamp (Last updated:) near the top of any docs you modify.
- Place files in canonical directories; avoid creating new top-level folders.
