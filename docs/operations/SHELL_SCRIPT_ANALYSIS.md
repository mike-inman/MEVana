# Shell Script Organization Analysis
Last updated: 2025-09-15
**Date:** 2025-09-12  
**Status:** 🔍 INVESTIGATION COMPLETE - Organization Needed

## 📊 Current Shell Script Distribution

### **Location Summary**
- **Root level**: 12 scripts (scattered)
- **scripts/**: 18 scripts (organized)  
- **infrastructure/scripts/**: 42+ scripts (mixed purpose)
- **Service-specific**: 3 scripts in crate directories
- **tools/**: 1 script
- **Total**: **53+ shell scripts** across multiple locations

## 🔍 **Key Findings & Issues**

### **1. Scattered Organization**
```
Root Level (should be minimal):
├── deploy.sh                    # Vultr deployment
├── deploy_fast.sh              # Fast deployment variant  
├── server_deploy.sh            # Server-specific deployment
├── direct_deploy.sh            # Direct deployment
├── deploy_monitoring.sh        # Monitoring deployment
├── start_services.sh           # Service startup (DUPLICATE?)
├── stop_services.sh            # Service shutdown
├── health_check.sh             # Health monitoring
├── quick_health_check.sh       # Quick health check
├── monitor.sh                  # System monitoring  
├── setup-live-trading.sh       # Trading setup
└── update_server.sh            # Server updates
```

### **2. Potential Duplicates Identified**

**Start Scripts:**
- `./start_services.sh` (root) vs `./scripts/start_system.sh` (organized)
- Purpose overlap: Both start MEVana services
- **Recommendation**: Consolidate to `scripts/start_system.sh` (better organized)

**Deployment Scripts:**
- `./deploy.sh` (Vultr-specific)
- `./deploy_fast.sh` (Fast variant)
- `./server_deploy.sh` (Server-specific)
- `./direct_deploy.sh` (Direct variant)
- `./infrastructure/scripts/deploy.sh` (Infrastructure)
- **Recommendation**: Consolidate into `scripts/deployment/` subdirectory

**Health Check Scripts:**
- `./health_check.sh` (comprehensive)
- `./quick_health_check.sh` (quick version)
- **Recommendation**: Keep both, move to `scripts/monitoring/`

### **3. Infrastructure Scripts Analysis**
The `infrastructure/scripts/` directory contains **42+ scripts** with mixed purposes:

**Categories Found:**
```
AI/Testing Scripts:
├── ai-intelligence-*.sh (3 variants)
├── test-*.sh (9 testing scripts)
├── multi-agent-collaborative-development-system.sh

Development Scripts:
├── dev-*.sh (start, stop, run variants)
├── run-dev.sh
├── seed-dev.sh

Utility Scripts:
├── concat-repo.sh
├── create-sitemap.sh  
├── notify-discord.sh
├── sitemap.sh

Infrastructure:
├── create-topics.sh
├── deploy.sh
└── test-heartbeats.sh
```

### **4. Service-Specific Scripts**
```
├── crates/edge-assembler/start_edge_assembler.sh
├── crates/normalizer-service/scripts/deploy-normalizer.sh  
└── tools/latency_test.sh
```

## 🎯 **Recommended Organization Structure**

### **Proposed `scripts/` Directory Structure:**
```
scripts/
├── deployment/
│   ├── deploy-vultr.sh          # (from ./deploy.sh)
│   ├── deploy-fast.sh           # (from ./deploy_fast.sh)
│   ├── deploy-server.sh         # (from ./server_deploy.sh)
│   ├── deploy-direct.sh         # (from ./direct_deploy.sh)
│   ├── deploy-monitoring.sh     # (from ./deploy_monitoring.sh)
│   └── setup-live-trading.sh    # (from ./setup-live-trading.sh)
├── monitoring/
│   ├── health-check.sh          # (from ./health_check.sh)
│   ├── quick-health-check.sh    # (from ./quick_health_check.sh)
│   ├── monitor.sh               # (from ./monitor.sh)
│   └── metrics_probe.sh         # (existing)
├── lifecycle/
│   ├── start_system.sh          # (keep existing, remove duplicate)
│   ├── stop_system.sh           # (rename from stop_services.sh)
│   ├── shutdown_system.sh       # (existing)
│   └── start_clean.sh           # (existing)
├── testing/
│   ├── run-end-to-end-tests.sh  # (existing)
│   ├── test-*.sh                # (from infrastructure/scripts/)
│   └── local-ci-check.sh        # (existing)
├── maintenance/
│   ├── backup_postgres.sh       # (existing)
│   ├── docker-data-purge.sh     # (existing)
│   ├── update_server.sh         # (from root)
│   └── deploy-rollback.sh       # (existing)
└── development/
    ├── load-env.sh              # (existing)
    ├── smart-deploy.sh          # (existing)
    └── deploy-doctor.sh         # (existing)
```

### **Root Level Should Only Contain:**
```
Root/
├── README.md
├── CLAUDE.md  
├── Cargo.toml
├── docker-compose.yml
└── (essential project files only)
```

## 🔧 **Infrastructure Scripts Cleanup**

The `infrastructure/scripts/` directory needs major cleanup:

**Keep (Infrastructure-Related):**
- `create-topics.sh` (Kafka setup)
- `test-heartbeats.sh` (Infrastructure testing)
- `deploy.sh` (if infrastructure-specific)

**Move to Main Scripts:**
- All `dev-*.sh` scripts → `scripts/development/`
- All `test-*.sh` scripts → `scripts/testing/`
- AI and utility scripts → `scripts/utilities/` or archive

**Archive/Remove:**
- `concat-repo.sh`, `sitemap.sh` (development utilities)
- Multiple AI variant scripts (consolidate or archive)
- Discord notification scripts (if unused)

## ⚠️ **Immediate Actions Needed**

### **High Priority:**
1. **Remove duplicate start scripts** - Consolidate to `scripts/start_system.sh`
2. **Move deployment scripts** from root to `scripts/deployment/`
3. **Organize monitoring scripts** into `scripts/monitoring/`

### **Medium Priority:**
1. **Clean up infrastructure/scripts/** - 70% reduction possible
2. **Standardize script naming** - Consistent kebab-case
3. **Add script documentation** - Purpose and usage headers

### **Low Priority:**
1. **Create script index** - README.md in scripts/ directory
2. **Add version control** - Track script changes properly
3. **Implement script testing** - Ensure all scripts work

## 📋 **Benefits of Reorganization**

1. **Reduced root clutter** - 12 scripts → ~5 essential files
2. **Logical organization** - Purpose-based directory structure  
3. **Easier maintenance** - Scripts grouped by function
4. **Better documentation** - Clear script hierarchy
5. **Eliminated duplicates** - Single source of truth for functionality

## 🚨 **Risk Assessment**

**Low Risk Changes:**
- Moving scripts to subdirectories (paths can be updated)
- Removing obvious duplicates
- Renaming for consistency

**Medium Risk Changes:**
- Consolidating similar scripts
- Updating script references in documentation

**High Risk Changes:**
- Removing unused scripts (verify no dependencies first)
- Changing core deployment workflows

## 📈 **Recommendation Priority**

**Phase 1 (Immediate):**
1. Create `scripts/` subdirectory structure
2. Move root-level scripts to appropriate subdirectories  
3. Remove duplicate `start_services.sh` from root
4. Update documentation references

**Phase 2 (Next Sprint):**
1. Clean up `infrastructure/scripts/` directory
2. Consolidate deployment script variants
3. Add script documentation headers
4. Create scripts/README.md index

**Phase 3 (Future):**
1. Implement script testing framework
2. Add version control for script changes
3. Create automated script validation
4. Establish script development standards

The current **53+ scattered scripts** can be reduced to ~**30 well-organized scripts** with clear purposes and locations.
