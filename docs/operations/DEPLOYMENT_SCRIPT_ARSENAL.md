# MEVana Deployment Script Arsenal

**Purpose**: Comprehensive inventory of all deployment scripts categorized by use case
**Updated**: 2025-09-19

## 🔍 **NEW ENHANCED DISCOVERY SCRIPT**

### **scripts/ci/comprehensive-infrastructure-discovery.sh** ⭐ **RECOMMENDED**
**Purpose**: Complete server state analysis before deployment
**What it analyzes**:
- Docker infrastructure (containers, networks, mevana_net)
- Prometheus (targets, config, metrics collection)
- Grafana (dashboards, datasources, volume mounts)
- MEVana services (all ports, metrics endpoints)
- Configuration files (volume paths, environment vars)
- Infrastructure alignment vs local dev

**Usage**:
```bash
# Run on server to understand current state
./scripts/ci/comprehensive-infrastructure-discovery.sh
```

## 🚀 **PRE-DEPLOYMENT SCRIPTS** (Run Before Deploy)

### **Critical Pre-Flight Checks**
```bash
# 1. Environment validation
./scripts/doctor.sh                                    # Basic system check
./scripts/development/deploy-doctor.sh                 # AI-powered deployment diagnostics
./scripts/ci/comprehensive-infrastructure-discovery.sh # COMPREHENSIVE state analysis ⭐

# 2. Infrastructure verification
./scripts/ci/verify-prometheus-targets.sh              # Prometheus targets health
./scripts/ci/validate-grafana-dashboards.sh            # Dashboard functionality
./scripts/lifecycle/check_conflicts.sh                 # Port/service conflicts

# 3. Build verification
./scripts/development/local-cargo-check.sh             # Local build check
./scripts/testing/local-ci-check.sh                    # CI pipeline simulation
```

### **Configuration Validation**
```bash
# Docker & networking
./scripts/ci/validate-deployment.sh                    # Overall deployment validation
./scripts/monitoring/quick_health_check.sh             # System health snapshot

# Dependency checks
./scripts/testing/test-shyft-apis.sh                   # External API connectivity
./scripts/testing/test-observability-pipeline.sh       # Monitoring pipeline
```

### **Pre-Deployment Discovery Questions**
Run these to answer: "What's currently on the server?"

1. **`comprehensive-infrastructure-discovery.sh`** - Complete analysis ⭐
2. **`verify-prometheus-targets.sh`** - Are monitoring targets working?
3. **`validate-grafana-dashboards.sh`** - Are dashboards functional?
4. **`deploy-doctor.sh`** - Are dependencies ready?

## 🔧 **DEPLOYMENT SCRIPTS** (Actual Deployment)

### **Primary Deployment Options**
```bash
# Via GitHub Actions (RECOMMENDED for server)
# - Trigger: self-hosted-deploy.yml workflow
# - Uses: /opt/MEVana as deployment path
# - Handles: Building, tagging, service restart

# Direct server deployment
./scripts/deployment/deploy-server.sh                  # Full server setup (new server)
./scripts/deployment/direct_deploy.sh                  # Quick deployment
./scripts/rapid_deploy.sh                              # Fast deployment
./scripts/simple-deploy.sh                             # Simplified deployment
```

### **Specialized Deployment**
```bash
# Cloud deployment
./scripts/deployment/deploy-vultr.sh                   # Vultr-specific
./scripts/deployment/deploy-vultr-compose.sh           # Vultr with compose

# Component-specific
./scripts/deployment/deploy_monitoring.sh              # Monitoring stack only
./scripts/deployment/build-and-ship.sh                 # Build and deploy
./scripts/development/build-images-locally.sh          # Local image building
```

### **Lifecycle Management**
```bash
# Clean deployment
./scripts/lifecycle/start_clean.sh                     # Fresh start
./scripts/lifecycle/start_system.sh                    # System startup
./scripts/lifecycle/shutdown_system.sh                 # Clean shutdown
./scripts/lifecycle/stop_system.sh                     # Stop services
```

## 📊 **POST-DEPLOYMENT SCRIPTS** (Verify Success)

### **Immediate Validation** (Run right after deploy)
```bash
# 1. Health verification
./scripts/ci/post-deployment-diagnostics.sh            # Comprehensive post-deploy check
./scripts/ci/quick-health-check.sh                     # Fast health check
./scripts/monitoring/quick_health_check.sh             # System health

# 2. Component verification
./scripts/ci/verify-prometheus-targets.sh              # Monitoring targets
./scripts/ci/validate-grafana-dashboards.sh            # Dashboard functionality
./scripts/ci/run-all-diagnostics.sh                    # Full diagnostic suite
```

### **Monitoring & Observability**
```bash
# Live monitoring
./scripts/monitoring/health_check.sh                   # Health monitoring
./scripts/monitoring/monitor.sh                        # System monitoring
./scripts/monitoring/metrics_probe.sh                  # Metrics collection test
./scripts/test_live_data_flow.sh                       # Data flow verification

# Dashboard setup
./scripts/auto_setup_dashboard.sh                      # Automated dashboard setup
./scripts/setup_grafana_dashboards.sh                  # Grafana setup
./scripts/monitoring/start_local_observability.sh      # Local observability
```

### **Testing & Validation**
```bash
# End-to-end testing
./scripts/testing/run-end-to-end-tests.sh              # Complete system test
./scripts/testing/test-validation-pipeline.sh          # Validation pipeline
./scripts/testing/latency_test.sh                      # Performance testing
```

## 🔧 **MAINTENANCE SCRIPTS** (Ongoing Operations)

### **System Maintenance**
```bash
# Data management
./scripts/maintenance/backup_postgres.sh               # Database backup
./scripts/maintenance/docker-data-purge.sh             # Clean Docker data
./scripts/maintenance/reset_port_allocations.sh        # Fix port conflicts

# System updates
./scripts/maintenance/update_server.sh                 # Server updates
./scripts/maintenance/replay_database.sh               # Database replay
```

### **Recovery & Rollback**
```bash
# Recovery operations
./scripts/maintenance/deploy-rollback.sh               # Deployment rollback
./scripts/geyser_recovery.sh                           # Geyser stream recovery

# Troubleshooting
./scripts/development/fix-dashboards.sh                # Fix dashboard issues
./scripts/reset_grafana_clean.sh                       # Reset Grafana
```

## 🎯 **RECOMMENDED DEPLOYMENT WORKFLOW**

### **Phase 1: Pre-Deployment Discovery**
```bash
# 1. Comprehensive server analysis
ssh server "cd /opt/MEVana && ./scripts/ci/comprehensive-infrastructure-discovery.sh"

# 2. Specific component checks
ssh server "cd /opt/MEVana && ./scripts/ci/verify-prometheus-targets.sh"
ssh server "cd /opt/MEVana && ./scripts/ci/validate-grafana-dashboards.sh"

# 3. Environment validation
ssh server "cd /opt/MEVana && ./scripts/doctor.sh"
```

### **Phase 2: Configuration Alignment**
```bash
# If discovery reveals issues:
# 1. Sync working local config to server
rsync -av infrastructure/ server:/opt/MEVana/infrastructure/

# 2. Fix specific issues found by discovery scripts
# 3. Re-run discovery to confirm fixes
```

### **Phase 3: Deployment**
```bash
# Use GitHub Actions workflow (RECOMMENDED)
# Trigger: self-hosted-deploy.yml
# This uses the existing CI/CD pipeline
```

### **Phase 4: Post-Deployment Validation**
```bash
# 1. Immediate validation
ssh server "cd /opt/MEVana && ./scripts/ci/post-deployment-diagnostics.sh"

# 2. Dashboard verification
ssh server "cd /opt/MEVana && ./scripts/ci/validate-grafana-dashboards.sh"

# 3. Data flow confirmation
ssh server "cd /opt/MEVana && ./scripts/test_live_data_flow.sh"
```

## 🚨 **CRITICAL INFRASTRUCTURE ALIGNMENT REQUIREMENTS**

Based on script analysis, these must match between local dev and server:

### **Volume Path Configuration**
```yaml
# Local working config:
- ./infrastructure/grafana/dashboards/curated:/etc/grafana/provisioning/dashboards
- ./infrastructure/grafana/datasources:/etc/grafana/provisioning/datasources

# Server MUST have same paths (fixed in local)
```

### **Dashboard Files**
```bash
# Local (WORKING):
infrastructure/grafana/dashboards/curated/mevana-system-status.json

# Server MUST have:
/opt/MEVana/infrastructure/grafana/dashboards/curated/mevana-system-status.json
```

### **Docker Network**
```bash
# Both environments MUST have:
docker network create mevana_net
```

### **Prometheus Configuration**
```bash
# Both MUST have same scrape targets:
infrastructure/prometheus.yml (11 jobs configured)
```

## 🎯 **SCRIPT PRIORITIZATION**

### **Tier 1: Critical (Use These First)**
1. `comprehensive-infrastructure-discovery.sh` - Complete server analysis
2. `verify-prometheus-targets.sh` - Monitoring verification
3. `validate-grafana-dashboards.sh` - Dashboard verification
4. `post-deployment-diagnostics.sh` - Post-deploy validation

### **Tier 2: Important (Use For Troubleshooting)**
1. `deploy-doctor.sh` - AI-powered diagnostics
2. `quick-health-check.sh` - Fast health verification
3. `run-all-diagnostics.sh` - Comprehensive diagnostics

### **Tier 3: Specialized (Use As Needed)**
1. Component-specific deployment scripts
2. Maintenance and recovery scripts
3. Testing and validation scripts

## 🔧 **Quick Command Reference**

```bash
# Pre-deployment discovery
./scripts/ci/comprehensive-infrastructure-discovery.sh

# Pre-deployment validation
./scripts/ci/verify-prometheus-targets.sh
./scripts/ci/validate-grafana-dashboards.sh
./scripts/doctor.sh

# Post-deployment verification
./scripts/ci/post-deployment-diagnostics.sh
./scripts/ci/quick-health-check.sh
./scripts/test_live_data_flow.sh

# Emergency troubleshooting
./scripts/development/deploy-doctor.sh
./scripts/ci/run-all-diagnostics.sh
```

This arsenal gives you complete visibility and control over your MEVana deployment process!