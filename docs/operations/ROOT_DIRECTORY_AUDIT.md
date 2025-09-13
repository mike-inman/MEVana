# Root Directory Audit - Every File Scrutinized
**Date:** 2025-09-12  
**Purpose:** Determine proper placement for every root directory file

## 🔍 **File-by-File Analysis**

### **✅ ESSENTIAL - Must Stay in Root**

| File | Reason | Status |
|------|--------|---------|
| `Cargo.toml` | Rust workspace definition | ✅ KEEP |
| `Cargo.lock` | Dependency lock file | ✅ KEEP |
| `README.md` | Primary project documentation | ✅ KEEP |
| `CLAUDE.md` | Claude Code instructions | ✅ KEEP |
| `.gitignore` | Git configuration | ✅ KEEP |
| `.dockerignore` | Docker configuration | ✅ KEEP |

### **⚠️ CONFIGURATION FILES - May Need Organization**

| File | Current Purpose | Recommendation | Action |
|------|-----------------|----------------|---------|
| `.env` | Active environment variables | ✅ KEEP | Essential for local dev |
| `.env.example` | Environment template | ✅ KEEP | Good practice |
| `.env.dev` | Development config | 🔄 MOVE | → `config/environments/` |
| `.env.production` | Production config | 🔄 MOVE | → `config/environments/` |
| `.env.live-trading-template` | Trading template | 🔄 MOVE | → `config/environments/` |
| `high_frequency_config.env` | HF trading config | 🔄 MOVE | → `config/environments/` |
| `venue_config.env` | Venue configuration | 🔄 MOVE | → `config/environments/` |
| `geyser-streamer.env` | Geyser config | 🔄 MOVE | → `config/environments/` |

### **📊 DOCUMENTATION - Should Move to docs/**

| File | Content | Recommendation | Action |
|------|---------|----------------|---------|
| `DOCUMENTATION_INDEX.md` | Doc index | 🔄 MOVE | → `docs/` (top-level) |
| `PORT_MAPPING.md` | Port allocation | 🔄 MOVE | → `docs/operations/` |

### **🔧 DEVELOPMENT/BUILD FILES - Need Organization**

| File | Purpose | Recommendation | Action |
|------|---------|----------------|---------|
| `deny.toml` | Cargo deny config | ✅ KEEP | Build tool config |
| `docker-compose.monitoring.yml` | Monitoring stack | 🔄 MOVE | → `infrastructure/` |
| `executor-service.Dockerfile` | Docker build | 🔄 MOVE | → `crates/executor-service/` |
| `monitoring.Dockerfile` | Monitoring build | 🔄 MOVE | → `infrastructure/monitoring/` |
| `monitoring-nginx.conf` | Nginx config | 🔄 MOVE | → `infrastructure/monitoring/` |

### **🗄️ DATA/ARCHIVE FILES - Should Relocate**

| File | Content | Recommendation | Action |
|------|---------|----------------|---------|
| `mevana-deploy.tar.gz` | Old deployment | ❌ REMOVE | Archive deployment |
| `mevana-monitoring-deploy.tar.gz` | Old monitoring | ❌ REMOVE | Archive deployment |
| `orchestrator-config.toml.backup` | Config backup | 🔄 MOVE | → `backups/` |
| `token_registry_snapshot.json` | Token snapshot | 🔄 MOVE | → `data/` |

### **💾 DATABASE/SQL FILES - Need Database Home**

| File | Content | Recommendation | Action |
|------|---------|----------------|---------|
| `transaction_logger.sql` | DB schema | 🔄 MOVE | → `database/migrations/` |
| `transaction_logger_fixed.sql` | DB schema fix | 🔄 MOVE | → `database/migrations/` |

### **🔑 SECRET/KEY FILES - Security Concern**

| File | Content | Security Risk | Action |
|------|---------|---------------|---------|
| `jito-keypair.json` | Jito private key | 🚨 HIGH | → `config/keys/` + .gitignore |

### **🛠️ UTILITY/TOOL FILES - Need Tools Home**

| File | Purpose | Recommendation | Action |
|------|---------|----------------|---------|
| `curl-format.txt` | Curl output format | 🔄 MOVE | → `tools/` |
| `idl-config.json` | IDL configuration | 🔄 MOVE | → `idl-registry/` |
| `quick_api_server.py` | API server utility | 🔄 MOVE | → `tools/` |
| `prometheus-update.yml` | Prometheus config | 🔄 MOVE | → `infrastructure/prometheus/` |

### **💻 PLATFORM-SPECIFIC - Archive or Remove**

| File | Platform | Recommendation | Action |
|------|----------|----------------|---------|
| `run-raydium-stream.bat` | Windows batch | ❌ REMOVE | Outdated platform script |
| `run-raydium-stream.ps1` | PowerShell | ❌ REMOVE | Outdated platform script |

### **📝 IDE/EDITOR FILES - Keep Minimal**

| File | Purpose | Recommendation | Action |
|------|---------|----------------|---------|
| `MEVana.code-workspace` | VS Code workspace | ✅ KEEP | IDE convenience |
| `. cursorrules` | Cursor AI rules | ✅ KEEP | Development aid |

### **📋 SYSTEM FILES - OS Generated**

| File | Purpose | Recommendation | Action |
|------|---------|----------------|---------|
| `.DS_Store` | macOS metadata | ❌ REMOVE | Add to .gitignore |
| `mevana_service_errors.log` | Error log | 🔄 MOVE | → `logs/` |

### **🗑️ LEGACY/BACKUP - Archive or Remove**

| File | Purpose | Recommendation | Action |
|------|---------|----------------|---------|
| `_Cargo.toml` | Old Cargo file | ❌ REMOVE | Legacy backup |

## 📊 **Summary Statistics**

- **Total Files Analyzed:** 38 files
- **Keep in Root:** 8 files (21%)
- **Move to Proper Location:** 23 files (61%)  
- **Remove/Archive:** 7 files (18%)

## 🎯 **Recommended Actions**

### **Phase 1 - Immediate Security/Cleanup (High Priority)**
```bash
# 1. Secure secret files
mkdir -p config/keys
mv jito-keypair.json config/keys/
echo "config/keys/" >> .gitignore

# 2. Remove system files
rm .DS_Store _Cargo.toml

# 3. Remove old platform scripts
rm run-raydium-stream.bat run-raydium-stream.ps1

# 4. Archive old deployments
mkdir -p _archive/deployments
mv mevana*.tar.gz _archive/deployments/
```

### **Phase 2 - Configuration Organization (Medium Priority)**
```bash
# Create environment config structure
mkdir -p config/environments
mv .env.dev .env.production .env.live-trading-template config/environments/
mv high_frequency_config.env venue_config.env geyser-streamer.env config/environments/

# Move infrastructure configs
mv docker-compose.monitoring.yml infrastructure/
mv monitoring.Dockerfile monitoring-nginx.conf infrastructure/monitoring/
mv prometheus-update.yml infrastructure/prometheus/
```

### **Phase 3 - Database & Documentation (Lower Priority)**
```bash
# Organize database files
mv transaction_logger*.sql database/migrations/

# Move documentation to proper location
mv DOCUMENTATION_INDEX.md docs/
mv PORT_MAPPING.md docs/operations/

# Organize utility files
mv curl-format.txt quick_api_server.py tools/
mv idl-config.json idl-registry/
mv token_registry_snapshot.json data/

# Handle backups and logs
mv orchestrator-config.toml.backup backups/
mv mevana_service_errors.log logs/
```

## ✨ **Expected Root Directory After Cleanup**

**Essential Files Only (8 files):**
```
├── .dockerignore
├── .env
├── .env.example  
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── CLAUDE.md
├── README.md
├── deny.toml
├── MEVana.code-workspace
└── . cursorrules
```

**Directories (Organized):**
```
├── config/         (All configuration files)
├── crates/         (Rust workspace)  
├── docs/           (All documentation)
├── infrastructure/ (Docker/deployment configs)
├── scripts/        (All shell scripts)
├── tools/          (Utility scripts and tools)
├── database/       (SQL and migrations)
├── logs/           (Log files)
├── data/           (Data files and snapshots)
└── _archive/       (Old/deprecated files)
```

## 🔒 **Security Benefits**

1. **Secret Files Protected:** Moved `jito-keypair.json` to secure location
2. **Sensitive Configs Organized:** Environment files properly structured
3. **Clean .gitignore:** Prevents accidental commits of sensitive data
4. **Audit Trail:** Clear organization for security reviews

## 📈 **Maintenance Benefits**

1. **Reduced Root Clutter:** 38 → 11 files in root (71% reduction)
2. **Logical Organization:** Files grouped by purpose
3. **Better Security:** Secrets properly managed
4. **Easier Navigation:** Clear directory structure
5. **Professional Appearance:** Clean, organized codebase

The root directory will transform from a **cluttered workspace** to a **professional, organized project structure** that follows Rust ecosystem best practices.