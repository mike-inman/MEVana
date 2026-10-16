# MEVana Docker Deployment Strategy

## Overview
This document captures our deployment strategy analysis, configuration findings, and production considerations for the MEVana Solana AI trading platform.

## Deployment Strategy Choice: docker-compose.core.yml

### Why We Chose This Approach
1. **Production Parity**: Matches production deployment strategy with proper containerization
2. **Service Isolation**: Each service runs in isolated containers with resource management
3. **Health Checks**: Built-in health monitoring and dependency management
4. **Network Architecture**: Uses `mevana_net` external network for proper service discovery
5. **Scalability**: Container-based approach supports future scaling and orchestration

### Alternative Approaches Considered
- **Native Binaries**: Faster startup but lacks production parity
- **docker-compose.local.yml**: Uses Alpine + mounted binaries (development only)
- **docker-compose.full.yml**: Uses GitHub Container Registry images (requires pre-built images)

## Configuration Files Analysis

### Infrastructure Stack (docker-compose.core.yml)
**Infrastructure Services:**
- Zookeeper: `2181` (internal coordination)
- Kafka: `9092` (internal), `29092` (external host access)
- Redis: `6379` (internal), `6380` (external to avoid conflicts)
- PostgreSQL: `5432` (internal), `5433` (external)
- Prometheus: `9090` (internal), `9091` (external to avoid conflicts)
- Grafana: `3000` (internal), configurable external via `.env`

**Custom MEVana Services:**
```yaml
hotpath-arb-engine:
  - Internal: 8088
  - Environment: METRICS_PORT=8088, GEYSER_ENDPOINT, X_TOKEN
  - ⚠️ MISSING: External port mapping

normalizer-service:
  - Internal: 8085
  - External: 9901:8085
  - Processes: dex-transactions → orca-swaps.parsed.v2

discovery-service:
  - Internal: 9102
  - External: 9102:9102
  - Function: Unknown program discovery

executor-service:
  - Internal: 9904
  - External: 9904:9904
  - Function: Trade execution orchestration
```

### Environment Configuration (.env)
```bash
X_TOKEN=<SHYFT_X_TOKEN>
ENDPOINT=https://grpc.ny.shyft.to
JUPITER_API_KEY=7ad95b98-4855-4a0e-bb2d-362e17aefdd6
GRAFANA_HOST_PORT=3300  # Grafana external port override
```

### Prometheus Configuration (prometheus.core.yml)
**Expected Service Names:**
- `hotpath-arb-engine:8088`
- `discovery-service:9102`
- `executor-service:9904`

## Configuration Mismatches Identified

### 1. Container Naming Convention Mismatch
**Scripts Expect:** `mevana-*` naming
- `mevana-hotpath:8088`
- `mevana-normalizer:8085`
- `mevana-discovery:9102`

**docker-compose.core.yml Creates:** `infrastructure-*` naming
- `infrastructure-hotpath-arb-engine-1`
- `infrastructure-normalizer-service-1`
- `infrastructure-discovery-service-1`

**Resolution:** Docker network DNS resolves service names correctly regardless of container names.

### 2. Missing Port Mappings
**Critical Issue:** hotpath-arb-engine has no external port mapping
- Prometheus can scrape via Docker network ✅
- No external localhost access ❌
- Potential conflict with native processes ⚠️

### 3. Environment Dependencies
Services depend on `.env` file values loaded by docker-compose, particularly:
- `X_TOKEN` (Geyser authentication)
- `ENDPOINT` (Geyser endpoint URL)
- `GRAFANA_HOST_PORT` (port override)

## Diagnostic Scripts Available

### Conflict Detection
```bash
./scripts/lifecycle/check_conflicts.sh
```
- Checks for port conflicts (8081,8088,8090,8091,8095)
- Detects multiple cargo run processes
- Validates single instance policy

### Prometheus Verification
```bash
./scripts/ci/verify-prometheus-targets.sh
```
- Validates target accessibility
- Expects `mevana-*` naming convention
- Provides health status for all targets

### Deployment Validation
```bash
./scripts/ci/validate-deployment.sh
./scripts/ci/post-deployment-diagnostics.sh
```
- End-to-end deployment verification
- Health checks across all services

### Deployment Doctor
```bash
./scripts/development/deploy-doctor.sh
```
- AI-powered deployment diagnostics
- Pre-flight checks for local and remote deployment
- Port conflict analysis
- Dependency health verification

## Production Versions

### Current Production Files
1. **docker-compose.server.yml**: Production-ready infrastructure
   - Uses specific container names (`*-service`)
   - Exposes standard ports
   - Includes proper health checks

2. **docker-compose.full.yml**: Complete production stack
   - Uses GitHub Container Registry images
   - Format: `ghcr.io/mike-inman/mevana-*:${MEVANA_TAG:-dev}`
   - Requires pre-built and published images

3. **prometheus.prod.yml**: Production Prometheus config
   - May have different target naming conventions
   - Optimized scrape intervals

### Production Deployment Pattern
```bash
# Combined production deployment
docker compose -p mevana \
  -f infrastructure/docker-compose.server.yml \
  -f infrastructure/docker-compose.full.yml \
  up -d --build
```

## Watch-Outs and Considerations

### 1. Service Discovery Resolution
- **Issue**: Script expectations vs actual container names
- **Impact**: Verification scripts may fail despite working services
- **Mitigation**: Update scripts or use service names for validation

### 2. Port Conflicts
- **Issue**: Native processes vs containerized services
- **Impact**: Services fail to start or bind ports
- **Mitigation**: Use conflict detection scripts before deployment

### 3. Environment Variable Propagation
- **Issue**: Services depend on `.env` file being in correct location
- **Impact**: Services start but fail authentication or connectivity
- **Mitigation**: Validate environment setup before deployment

### 4. Health Check Dependencies
- **Issue**: Services may start before dependencies are ready
- **Impact**: Cascading failures and restart loops
- **Mitigation**: Proper depends_on conditions with health checks

### 5. Resource Management
- **Issue**: Docker builds are resource-intensive and time-consuming
- **Impact**: Long deployment times, potential resource exhaustion
- **Mitigation**: Use pre-built images for production, monitor system resources

## Deployment Workflow

### Development Deployment (Current)
1. Build workspace: `cargo build --release --workspace`
2. Deploy infrastructure + services: `cd infrastructure && docker compose -f docker-compose.core.yml up -d --build`
3. Verify conflicts: `./scripts/lifecycle/check_conflicts.sh`
4. Check targets: `./scripts/ci/verify-prometheus-targets.sh`

### Production Deployment
1. Pre-flight check: `./scripts/development/deploy-doctor.sh`
2. Deploy stack: Combined server + full compose files
3. Validate: `./scripts/ci/validate-deployment.sh`
4. Monitor: `./scripts/ci/post-deployment-diagnostics.sh`

## Troubleshooting Common Issues

### Container Build Timeouts
- **Symptom**: Docker build hangs or times out
- **Solution**: Increase Docker memory allocation, check disk space
- **Command**: `docker system prune -a --volumes -f` to free space

### Service Discovery Failures
- **Symptom**: Services can't connect to each other
- **Solution**: Verify `mevana_net` network exists: `docker network create mevana_net`

### Prometheus Scraping Issues
- **Symptom**: Targets show as "down" in Prometheus
- **Solution**: Check service names in prometheus.core.yml match docker-compose service names

### Port Binding Failures
- **Symptom**: "Port already in use" errors
- **Solution**: Run conflict detection script and kill conflicting processes

## File Locations Reference

```
infrastructure/
├── docker-compose.core.yml      # Development deployment (current choice)
├── docker-compose.server.yml    # Production infrastructure
├── docker-compose.full.yml      # Production services
├── prometheus.core.yml          # Prometheus config
└── .env                        # Environment variables

scripts/
├── lifecycle/
│   ├── check_conflicts.sh      # Port/process conflict detection
│   ├── start_clean.sh         # Clean system startup
│   └── shutdown_system.sh     # Graceful shutdown
├── ci/
│   ├── verify-prometheus-targets.sh  # Target validation
│   ├── validate-deployment.sh        # End-to-end validation
│   └── post-deployment-diagnostics.sh # Health monitoring
└── development/
    ├── deploy-doctor.sh        # Pre-flight diagnostics
    └── fix-dashboards.sh      # Dashboard repair tool
```

## Historical Context

This deployment strategy was developed during a complete system rebuild where:
1. Native binaries were running with port conflicts
2. Docker containers needed proper naming for Prometheus scraping
3. Production parity was prioritized over development speed
4. Multiple deployment strategies were evaluated and consolidated

The choice of docker-compose.core.yml represents a balance between development usability and production readiness, with the understanding that diagnostic scripts may need updates to match the actual container naming conventions.

---

**Last Updated**: 2025-09-19
**Contributors**: Claude Code Assistant, MEVana Development Team
**Status**: Active deployment strategy in use