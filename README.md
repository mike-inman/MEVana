# MEVana - Solana AI Trading Platform

[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](docs/)
![Coverage](docs/badges/coverage.svg)

**MEVana** is a comprehensive Solana AI trading platform designed to capture MEV (Maximal Extractable Value) opportunities through real-time blockchain data streaming, intelligent transaction analysis, and automated arbitrage execution.

## 🚀 Quick Start

### **Prerequisites**
- Rust 1.70+
- Docker Desktop
- Git

### **Get Started**
```bash
# Clone the repository
git clone https://github.com/mike-inman/MEVana.git
cd MEVana

# Build the workspace
cargo build --workspace

# Start infrastructure services
cd infrastructure
docker-compose up -d
```

### **Run Services**
```bash
# Start Orca streaming service
cargo run -p dex-stream-orca

# Start normalizer service
cargo run -p normalizer-service
```

## 📚 Documentation

**📖 [Full Documentation](docs/)** - Comprehensive guides and references

**🎯 [Master Summary](docs/MASTER_SUMMARY.md)** - Single source of truth for the platform

**🏗️ [Architecture](docs/architecture/)** - System design and components

**⚡ [Arbitrage Strategy & Architecture (AI-friendly)](docs/AI_ARBITRAGE_STRATEGY_2025-08-21.md)** - Hot-path rules, Edge model, coherency, scaling

**👨‍💻 [Development](docs/development/)** - Setup and development guides

**🚀 [Deployment](docs/deployment/)** - Infrastructure and deployment

**📈 [Project Evolution](docs/PROJECT_EVOLUTION.md)** - Development timeline and phases

## 🏗️ Architecture

MEVana consists of several key components:

- **Streaming Services** - Real-time DEX transaction monitoring (Orca, Raydium, Meteora)
- **Normalizer Service** - IDL-based code generation and transaction parsing
- **Arbitrage Engine** - MEV opportunity detection and execution
- **Infrastructure Layer** - Kafka, Redis, PostgreSQL, monitoring

## 🛠️ Technology Stack

- **Backend**: Rust, Tokio, Solana SDK
- **Infrastructure**: Docker, Kafka, Redis, PostgreSQL
- **Monitoring**: Prometheus, Grafana
- **Protocols**: Whirlpool, Raydium CLMM, Meteora DAMM

## ♻️ Recent Arbitrage Engine Enhancements (August 2025)

The arbitrage / MEV engine recently gained several adaptive intelligence and observability features:

- Adaptive Position Sizing: Median profit-driven multiplier with min/max bounds and floor size
- Depth & Slippage-Aware Quoting: Linear depth impact model producing adjustment factor per leg
- Unified Evaluation Path: Consolidated 2-leg / 3-leg evaluation with integrated depth + sizing
- Persistence: Adaptive sizing state periodically persisted to JSON (crash-safe recovery)
- Runtime Introspection: `/ai/sizing` HTTP endpoint surfaces current adaptive state + history snapshot
- Expanded Metrics: Size utilization, depth adjustment factor histograms, adaptive multiplier gauge

### Key Environment Variables (Arb Engine)

| Variable | Purpose |
|----------|---------|
| `ARB_SIZING_MODE` | `adaptive` or `fixed` sizing strategy selector |
| `ARB_ADAPT_HISTORY` | Number of recent profit samples retained for median/multiplier calc |
| `ARB_ADAPT_MIN_MULTIPLIER` / `ARB_ADAPT_MAX_MULTIPLIER` | Bounds on adaptive size multiplier |
| `ARB_ADAPT_MIN_SIZE_LAMPORTS` | Floor notional (lamports) to enforce minimum trade size |
| `ARB_DEPTH_PRICE_IMPACT_BPS` | Linear per-unit size depth impact (basis points) used in quoting |
| `ARB_MIN_PROFIT_BPS` | Minimum net profit threshold after adjustments |
| `ARB_MAX_SLIPPAGE_BPS` | Maximum allowed slippage before rejecting opportunity |
| `ARB_ADAPT_STATE_PATH` | Optional path to persist adaptive state JSON (enables warm restart) |
| `ARB_ADAPT_PERSIST_INTERVAL` | Minimum seconds between state persistence writes |

HTTP Introspection now includes: `/metrics`, `/ai/config{, /reload}`, `/ai/advice`, `/ai/sizing`.

> Tip: Point Prometheus at the metrics endpoint to scrape the new histograms and gauges for multiplier, depth factor distribution, and size utilization.

## 🔧 Development

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Check code quality
cargo clippy --workspace
cargo fmt --all
```

## 📊 Performance

- **Throughput**: 10,000+ transactions/second
- **Latency**: <100ms end-to-end processing
- **Availability**: 99.9% uptime target

## 📈 Executor Metrics Overview

Prometheus metrics (executor-service) – selected signals:

| Category | Metric | Description |
|----------|--------|-------------|
| Profit | mevana_realized_profit_lamports_total | Sum of realized profits (lamports) |
| Profit | mevana_profit_overestimation_lamports_total | Overestimated portion (est > realized) |
| Profit | mevana_profit_underestimation_lamports_total | Underestimated portion (realized > est) |
| Profit | mevana_profit_slippage_lamports_total | Aggregated absolute slippage (direction split above) |
| Bundles | mevana_bundle_submissions_total | Jito bundle submissions |
| Bundles | mevana_bundle_confirmations_total | Landed bundles |
| Bundles | mevana_bundle_unlanded_total | Submitted but not landed in window |
| Bundles | mevana_bundle_failed_submit_total | Failed pre-polling |
| Accounts | mevana_account_sets_missing | Dynamic missing account sets (computed) |
| Accounts | mevana_account_sets_required_missing | Hard-coded required keys missing (MISSING_REQUIRED) |
| Token Meta | /token/{mint} (HTTP) | Lightweight JSON metadata endpoint (port TOKEN_REGISTRY_PORT, default 9898) |

Set `TOKEN_REGISTRY_PORT` to expose the token metadata endpoint (default 9898). Scrape `/metrics` for all counters/gauges.


### Account Registry Versioning & Validation

Versioned registry manifests live in `profiles/` (e.g. `account_registry.v1.json`). The placeholder `account_registry.example.json` is **excluded** from CI diff / strict gating. Always diff between real `vN` files.

Validate one or more files strictly:
```
cargo run -p executor-service --bin account-registry-validate -- --strict profiles/account_registry.v1.json profiles/account_registry.v2.json
```

Generate a structured diff + JSON Patch between two versions (produces `diff.json`):
```
cargo run -p executor-service --bin account-registry-validate -- \
	--diff --patch-json diff.json profiles/account_registry.v1.json profiles/account_registry.v2.json
```

Key behaviors:
- Independent validation per file (`VALID` / `INVALID` lines)
- Non-zero exit (code 2) on validation errors or unauthorized decimal decreases
- JSON Patch + human diff (added/removed entries, decimal changes) when `--diff`
- Optional decimal decrease allowlist via file (`profiles/decimal_decrease_allowlist.txt`) + env (`DECIMAL_DECREASE_ALLOWLIST`)

Schema: `executor-service/account_registry.schema.json` (Draft-07). Optionally run an external linter before invoking the built-in validator.

Backlog / Planned Enhancements:
- Provenance metadata & cryptographic signature for promoted versions
- AI-assisted suggested patch when unknown mints appear at runtime

See `profiles/README.md` for full promotion workflow & policy details.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/mike-inman/MEVana/issues)
- **Discussions**: [GitHub Discussions](https://github.com/mike-inman/MEVana/discussions)

---

**MEVana** - Capturing MEV opportunities on Solana with AI-powered intelligence.
