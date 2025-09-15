# CI/CD Pipeline Plan (Initial Draft)
Last updated: 2025-09-15

Goal: Provide fast feedback (<5 min) on critical paths (build, lint, unit tests), enforce registry/schema validation, and enable progressive delivery (staging -> canary -> prod) with guardrails for MEV / trading safety.

## Stages Overview
1. Pre-commit / Local (optional hooks)
   - rustfmt + clippy (deny warnings in touched crates)
   - fast unit tests (`cargo test --workspace --lib -- --nocapture`)
2. CI - Pull Request
   - Matrix build (stable, nightly) minimal features
   - Feature-gated build (strict-accounts, diagnostics) to surface latent issues
   - Security / supply chain: cargo-deny (advisories, duplicates, bans)
   - Schema & registry validation: `account-registry-validate --strict --diff profiles/account_registry.example.json` plus any submitted new versions under `profiles/pending/`
   - Lints: clippy (deny warnings except explicitly allowed TODO markers)
   - Unit + doc tests
   - (Future) deterministic integration tests for hotpath edge detector (tagged, opt-in on label)
3. CI - Merge to main
   - All PR steps
   - Build and push container images (executor, arb-engine, services) with git SHA tags
   - Publish SBOM (cyclonedx) for each image
   - Upload coverage (grcov or cargo-llvm-cov) threshold gate (e.g. >= 60%)
4. Staging Deploy (automated)
   - Apply k8s manifests / docker compose with staging configs (reduced risk params)
   - Run smoke tests: ping `/metrics`, simulate single arbitrage opportunity (dry-run mode), validate critical metrics presence
5. Canary Prod Deploy (manual approval step)
   - Deploy single replica / small fraction of traffic (hotpath WS subscriber in observe-only mode)
   - Observe metrics for N blocks (error rates, circuit open status, bundle failure ratio)
6. Full Prod Deploy
   - Scale to desired replicas
   - Post deploy verification: realized profit counter increasing OR controlled no-op if market closed; no sustained circuit breaker open.

## Key Jobs (GitHub Actions Skeleton)
- ci-build.yml: build + test + clippy + deny + registry validation
- ci-images.yml: build & push container images on main
- deploy-staging.yml: triggered on successful main build; uses OIDC -> cloud
- deploy-prod.yml: requires environment protection + approval

## Fast-Fail Priorities
1. Syntax / compile
2. Clippy (deny) & formatting
3. Registry/schema validation
4. Unit tests
5. Integration / slower tests (feature flagged or label triggered)

## Test Taxonomy
- Unit: pure logic (profit calc, registry diff, circuit breaker, edge detection)
- Deterministic integration: simulated edges -> verify signals
- Non-deterministic / network (Shyft, Jito) mocked behind feature (excluded by default)

## Artifacts & Promotion
- Registry versions placed in `profiles/pending/` with semantic-ish timestamp names; validator diff run vs last promoted (`profiles/account_registry.vCURRENT.json`). Promotion merges rename or copy into `profiles/active/`.
- Attach diff report to PR (Actions step writes markdown summary comment).

## Rollback Strategy
- Keep previous N container image tags (by SHA & date)
- `DEPLOY_VERSION` environment variable selects image tag in manifests
- Rollback = redeploy last known good tag + revert config map (min profit, fee params)

## Safety Gates
- Min profit threshold configurable via env + enforced in simulation stage (already implemented)
- Circuit breaker metrics exported; deployment blocks if open > X% of last 10 minutes in staging test
- Profit slippage ratio p95 difference vs last 7-day baseline (future anomaly job)

## Next Steps
- [x] Add GitHub Action workflow skeletons (initial ci.yml in place)
- [x] Add cargo-deny configuration (deny.toml + workflow step)
- [x] Add cargo-llvm-cov (coverage artifact produced; gating threshold pending)
- [x] Implement registry diff markdown output (GitHub Summary integration + JSON artifact)
- [x] Containerize executor-service (executor-service.Dockerfile; other services pending)
- [ ] Add coverage gating threshold (e.g. fail if < 60%)
- [ ] Add smoke test binary (simulate single opportunistic cycle) for staging
- [ ] Add arb-engine & other service Dockerfiles
- [ ] Publish SBOM for images (cyclonedx cargo + syft)
- [ ] Add decimal decrease allowlist file & CI gating (basic env allowlist implemented)

### Recent Additions
Validator now enforces a safety gate: any token mint decimal decrease across registry versions causes CI failure unless the mint appears in `ACCOUNT_REGISTRY_DECIMAL_DECREASE_ALLOW` (comma-separated). This reduces risk of unintended precision shrinkage impacting downstream balance normalization.

