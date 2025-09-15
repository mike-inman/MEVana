Rust Coding Conventions (MEVana)

Last updated: 2025-09-15

Naming
- Types/traits: CamelCase (e.g., TokenRegistry, PriceOracle)
- Functions/vars/modules: snake_case (e.g., load_snapshot_file, token_count)
- Consts/statics: SCREAMING_SNAKE_CASE (e.g., MAX_RETRY_ATTEMPTS)
- Feature flags: kebab-case in Cargo.toml (e.g., "metrics-export")

Formatting & Lints
- Formatting: `cargo fmt --all` (rustfmt)
- Lints: `cargo clippy --workspace --all-targets -- -D warnings`
- Allow-lints only with justification and the narrowest scope possible

Modules & Imports
- One module per concern; avoid deep nesting
- Re-export only when it improves ergonomics (pub use) from lib.rs
- Group imports; avoid wildcard imports except obvious preludes

Error Handling
- Return `Result<T, E>`; use `?` to propagate
- Prefer domain-specific errors via `thiserror`; use `anyhow` at boundaries
- Do not `unwrap()`/`expect()` in production code (allowed in tests and short-lived tools)

Logging & Observability
- Use `tracing` (not println!) with structured fields
- Add spans for async workflows and high-latency operations
- Emit error contexts (`with_context` / `map_err`) for failures

Concurrency & Async
- Use `tokio` async; do not block within async (use spawn_blocking or dedicated threads)
- Prefer channels and cancellation-friendly designs; set timeouts and backoffs
- Avoid unbounded tasks; name tasks and handle JoinError

Safety & Performance
- Avoid `unsafe`; if required, encapsulate, document, and test thoroughly
- Prefer slices/iterators over temporary Vec allocations; preallocate when sizes are known
- Be explicit about integer sizes and endianness in protocol code

Configuration & Env
- Read config from env or config files; validate early
- Keep operational defaults safe; document all env vars in docs where used

APIs & Public Surfaces
- Keep public APIs minimal; hide internals via modules
- Add rustdoc (`///`) with Examples and Errors sections

Testing
- Unit tests co-located (mod tests); integration tests under `tests/`
- Mark heavy/integration tests with `#[ignore]` and document how to run
- Prefer deterministic tests; use proptest for critical invariants where helpful

Crate Layout
- `lib.rs` for library; keep `main.rs` thin and focused on wiring
- Separate data types (`types.rs`) and traits (`traits.rs`) where large

Commit Discipline
- Small, focused changes; include rationale in commit messages
- Keep changes aligned with these conventions unless there’s a compelling reason (document it)

