# MEVana Scripts Directory
**Organized Structure for All Shell Scripts**

## 📂 Directory Structure

### `deployment/` - Deployment Scripts
- `deploy-vultr.sh` - Main Vultr cloud deployment
- `deploy_fast.sh` - Fast deployment variant
- `deploy-server.sh` - Server-specific deployment  
- `direct_deploy.sh` - Direct deployment method
- `deploy_monitoring.sh` - Monitoring stack deployment
- `setup-live-trading.sh` - Live trading environment setup

### `lifecycle/` - Service Lifecycle Management
- `start_system.sh` - **Primary system startup script**
- `start_clean.sh` - Clean system startup (kills conflicts)
- `stop_system.sh` - Graceful system shutdown
- `shutdown_system.sh` - Emergency system shutdown

### `monitoring/` - Health & Monitoring
- `health_check.sh` - Comprehensive health check
- `quick_health_check.sh` - Fast health verification
- `monitor.sh` - Continuous system monitoring
- `metrics_probe.sh` - Metrics collection testing

### `testing/` - Testing & Validation
- `run-end-to-end-tests.sh` - Full system testing
- `local-ci-check.sh` - Local CI/CD validation
- `test-observability-pipeline.sh` - Observability testing
- `test-shyft-apis.sh` - Shyft API validation
- `test-tickarray-fallback.sh` - Tick array testing
- `test-validation-pipeline.sh` - Validation pipeline testing

### `maintenance/` - System Maintenance
- `backup_postgres.sh` - Database backup
- `docker-data-purge.sh` - Clean Docker volumes/data
- `deploy-rollback.sh` - Rollback deployment
- `update_server.sh` - Server updates

### `development/` - Development Tools
- `load-env.sh` - Load development environment
- `smart-deploy.sh` - Intelligent deployment
- `deploy-doctor.sh` - Deployment diagnostics

## 🚀 Quick Start

### Start the System
```bash
./scripts/lifecycle/start_system.sh
```

### Health Check
```bash
./scripts/monitoring/health_check.sh
```

### Deploy to Production
```bash
./scripts/deployment/deploy-vultr.sh
```

### Run Tests
```bash
./scripts/testing/run-end-to-end-tests.sh
```

### Stop the System
```bash
./scripts/lifecycle/stop_system.sh
```

## ✅ Organization Benefits

1. **Eliminated Root Clutter**: Moved 12 scripts from root directory
2. **Logical Grouping**: Scripts organized by purpose
3. **Removed Duplicates**: Consolidated duplicate start/deployment scripts  
4. **Consistent Naming**: Standardized script naming conventions
5. **Better Discovery**: Easy to find scripts by function

## 📋 Migration Notes

**Scripts Moved from Root:**
- `deploy.sh` → `deployment/deploy-vultr.sh`
- `start_services.sh` → **REMOVED** (duplicate of `lifecycle/start_system.sh`)
- `health_check.sh` → `monitoring/health_check.sh`
- `monitor.sh` → `monitoring/monitor.sh`
- All deployment variants → `deployment/` directory

**Documentation Updates Needed:**
- Update CLAUDE.md references to new script paths
- Update Docker/deployment docs with new paths
- Update operational procedures with new structure

## 🔍 Still to Organize

**Infrastructure Scripts** (`infrastructure/scripts/` - 42+ scripts):
- Many development/testing scripts that could be moved to `scripts/testing/`
- AI utility scripts that might need archiving
- Docker/Kafka setup scripts that belong in `infrastructure/`

**Service-Specific Scripts**:
- `crates/edge-assembler/start_edge_assembler.sh`
- `crates/normalizer-service/scripts/deploy-normalizer.sh`
- `tools/latency_test.sh`

## 💡 Usage Tips

1. **Always use lifecycle scripts** for system management
2. **Run health checks** before and after deployments  
3. **Test locally** before production deployments
4. **Check script permissions** - all should be executable (`chmod +x`)
5. **Use absolute paths** when calling from other scripts