## 2025-08-27 — Local run + env loader

- Added `scripts/load-env.sh` to load tokens from `~/.mevana.env` or `infrastructure/.env` once per shell.
- Added `crates/dex-stream-universal/Dockerfile.slim` and wired `whirlpool-tx-stream` to use it.
- `whirlpool-tx-stream` runs at `http://localhost:8091` (health), producing to `dex-transactions`.
- Next: confirm Kafka flow for Whirlpool and wire a runtime-only Docker image if needed.


