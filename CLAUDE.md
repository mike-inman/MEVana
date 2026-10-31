# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Commands

### Build & Development
**Toolchain (2026-10-30):** always use rustup's cargo (`~/.cargo/bin/cargo`), which honors `rust-toolchain.toml` (`nightly-2025-09-15`).
- Homebrew Rust 1.98.1 is also installed at `/usr/local/bin/cargo` and ignores the pin. If `cargo -V` does not show `1.92.0-nightly`, put `~/.cargo/bin` first on `PATH`, or run `~/.cargo/bin/cargo`.
- Validation results from the Homebrew compiler are invalid. Example: an `ethnum` E0512 error.

```bash
# Build entire workspace
cargo build --workspace

# Build specific high-performance targets
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --release -p hotpath_arb_engine

# Run tests
cargo test --workspace

# Code quality checks (run before committing)
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
./scripts/ci/repo_hygiene.sh
```

### System Operations
```bash
# Start complete system (Docker infrastructure + services)
./scripts/lifecycle/start_system.sh

# Start infrastructure only
cd infrastructure && docker-compose up -d

# Run main HotPath arbitrage engine
PAPER_TRADING=true DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" X_TOKEN="<SHYFT_X_TOKEN>" \
METRICS_PORT=8088 RUST_LOG=info cargo run -p hotpath_arb_engine --bin hotpath_arb_engine

# Graceful system shutdown
./scripts/lifecycle/shutdown_system.sh
```

### Health Monitoring
```bash
# Check service health
curl -s localhost:8088/performance  # HotPath metrics
curl -s localhost:8085/metrics      # Arbitrage engine
curl -s localhost:8091/health       # Monitoring service

# Check running processes
ps aux | grep -E "(hotpath|executor|normalizer|discovery)"
docker ps
```

## Architecture Overview

MEVana is a Solana MEV platform with three distinct architectures:

### 1. HotPath Engine (Primary - Zero Latency)
- **Location**: `crates/hotpath_arb_engine/`
- **Purpose**: Direct Geyser streaming for ultra-low latency MEV detection
- **Key Features**: No Kafka dependency, direct transaction processing, sub-100ms latency
- **Run**: Use environment variables above with `hotpath_arb_engine` binary

### 2. Legacy Kafka Pipeline
- **Components**: normalizer-service → Kafka → edge-assembler → arb-engine
- **Status**: Being phased out in favor of HotPath
- **Use Case**: Historical data analysis and batch processing

### 3. Supporting Services
- **executor-service**: Trade execution with Jito bundle support
- **monitoring-service**: Metrics aggregation and API endpoints
- **wallet-vault**: Secure wallet management with Clerk auth
- **api-service**: REST API for frontend integration

## Critical Configuration

### Database
- PostgreSQL required: `postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb`
- Migrations auto-applied on service startup
- Schema: `docs/database/complete_schema.sql`

### Environment Variables (HotPath)
```bash
PAPER_TRADING=true                    # Enable paper trading mode
DATABASE_URL=...                      # PostgreSQL connection
GEYSER_ENDPOINT="https://grpc.ny.shyft.to"  # Shyft RPC endpoint
X_TOKEN="<SHYFT_X_TOKEN>"  # Auth token
METRICS_PORT=8088                     # Prometheus metrics port
MULTI_ENDPOINT_MODE=true              # Enable multiple Geyser endpoints
```

### Ports (Production)
- 8088: HotPath metrics
- 8085: Arbitrage WebSocket
- 8091: Monitoring API
- 5433: PostgreSQL
- 29092: Kafka
- 6380: Redis
- 3000: Grafana

## Development Guidelines

### File Organization
- **Dashboards**: `infrastructure/grafana/dashboards/`
- **Scripts**: `scripts/{deployment,lifecycle,monitoring,maintenance,testing}/`
- **Documentation**: `docs/{operations,deployment,architecture,strategy,reference}/`
- **Service Code**: `crates/{service-name}/`

### Code Style Requirements
- Run `cargo fmt` before commits
- Fix all `cargo clippy` warnings
- Follow patterns in `docs/development/RUST_STYLE.md`
- Use existing error handling patterns (anyhow for apps, custom errors for libs)

### Testing Requirements
- Write tests for new functionality
- Run `cargo test --workspace` before pushing
- Integration tests in `tests/` directories
- Use `PAPER_TRADING=true` for safe testing

### Git Workflow
```bash
# Check current status
git status

# Run pre-commit checks
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/ci/repo_hygiene.sh

# Create commits with descriptive messages
git add -A
git commit -m "feat: Add adaptive position sizing to HotPath engine"
```

## Important Context

### Current Focus (updated 2026-10-29): Stage 0 research track
The project is in a **research phase with trading disabled**. The question being answered is: *does ranked route selection beat simple baselines on executable, cost-adjusted opportunities?*

Work proceeds through dated handoffs in `docs/`. The latest one is the authority.

1. **Handoffs 07–08 (done):** research contracts (experiments, rankings, execution outcomes) in `dotnet/` plus OpenSpec.
2. **Handoff 09 (done):** read-only Geyser capture of Orca Whirlpool data (`crates/geyser-capture`).
   - Providers are Sodae (primary) and Tatum (secondary).
   - Follow the stream-discipline rules: no idle streams, and no quota wasted.
3. **Handoff 10 (done):** exact Whirlpool quote math (`whirlpool_core::exact`, Apache Orca core pinned at 1.0.4). Quotes are bounded to the supplied tick arrays.
4. **Handoff 11 (done, 2026-10-30):** deterministic offline replay (`crates/research-replay`), producing three per-pair `Simulated` results (SOL/USDC, SOL/USDT, USDC/USDT).
   - No positive opportunity was found in the 10-minute capture, and missing state was heavy.
   - See `docs/2026-10-30-0828-CDT-research-handoff-11-closeout.md`.
5. **Handoff 12 (done, 2026-10-30):** slot-correct replay (Confirmed state, fork writes dropped, skipped slots are not gaps) and bounded Tatum RPC tick-array snapshots.
   - SOL/USDC and USDC/USDT are now about 99.7% evaluable; SOL/USDT is about 45%.
   - See `docs/2026-10-30-1139-CDT-research-handoff-12-closeout.md`.
6. **Handoff 13 (done, 2026-10-30):** experiment request contract v2, for multi-pair route universes and triangles. It is .NET plus OpenSpec only, and v1 stays frozen.
   - See `docs/2026-10-30-1325-CDT-research-handoff-13-request-v2-closeout.md`.
7. **Handoff 14 (done, 2026-10-30):** offline replay of the 152-route USDC universe, with a v2-linked `Simulated` result.
   - 69.81% of triangle candidates were evaluable, and none was profitable. The two detected opportunities were two-leg, and both simulated negative.
   - See `docs/2026-10-30-1618-CDT-research-handoff-14-multi-route-replay-closeout.md`.
8. **Handoff 15 (done, 2026-10-30):** read-only, loopback-only Research API on port 8098, serving the four validated experiments. Handoff 15a pins all 11 seed files by SHA-256, so any seed change needs a deliberate pin update.
   - See `docs/2026-10-30-2123-CDT-research-handoff-15-read-only-research-api-closeout.md`.
9. **Handoff 16 (done, 2026-10-31):** signed-in MEVana-UI research pages at `/research`, read from the loopback API by server code. It is tables only and works locally; the deployed UI shows "Research API unavailable" until the API is deployed with real authentication.
   - See the closeout in MEVana-UI: `docs/2026-10-30-2234-CDT-research-handoff-16-closeout.md`.
10. **Next:**
    - handoff 17: Grafana research evidence, which is now the standard for visualization (`docs/2026-10-31-0920-CDT-research-handoff-17-grafana-research-evidence.md`);
    - handoff 18: indicative price-gap survey across Orca, Raydium CLMM and Meteora DLMM, with live runs finished by 2026-11-04 (`docs/2026-10-31-0935-CDT-research-handoff-18-price-gap-survey.md`).
    - Later: exact modeling and a server-host capture for segments that pass the survey gate.

**Not in focus right now:**
- HotPath latency work, executor or Jito, wallets, live or paper trading;
- the Kafka pipeline, ML sizing, multi-region and Jupiter.

Do not change `hotpath_arb_engine`, `executor-service` or `shared-core` unless a handoff says to. Implement directly in this repo with OpenSpec. Do not use the old AgentEngineering factory process.

### Known Issues
- Kafka consumer lag during high throughput (legacy pipeline)
- PostgreSQL connection pool exhaustion under load
- Geyser reconnection needs improvement for network interruptions

### Performance Targets
- End-to-end latency: <100ms
- Transaction throughput: 10,000+ TPS
- Profit margin: 5-25 bps minimum
- System uptime: 99.9%

### Security Considerations
- Never commit `.env` files with real credentials
- Use `PAPER_TRADING=true` for all development
- Wallet keys stored in `wallet-vault` service only
- All production configs in `config/environments/`

## Quick Debugging

### Service Won't Start
```bash
# Check ports
lsof -i :8088 -i :8085 -i :5433

# Check Docker services
docker ps
docker logs postgres

# Check database connection
PGPASSWORD=<POSTGRES_PASSWORD> psql -h localhost -p 5433 -U mev -d mevdb -c "SELECT 1"
```

### No Data Flow
```bash
# Check Geyser connection
curl -s localhost:8088/performance | jq .

# Check Kafka topics (legacy)
docker exec -it kafka kafka-topics.sh --list --bootstrap-server localhost:29092

# View logs
RUST_LOG=debug cargo run -p hotpath_arb_engine
```

### Build Failures
```bash
# Clean and rebuild
cargo clean
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --workspace

# Check for missing dependencies
cargo tree -p hotpath_arb_engine
```

## References

- **Master Documentation**: `docs/DOCUMENTATION_INDEX.md`
- **Self-Hosted Deploy**: `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md`
- **Architecture**: `docs/HOT_PATH_DESIGN.md`
- **Contributing**: `CONTRIBUTING.md` and `AGENTS.md`