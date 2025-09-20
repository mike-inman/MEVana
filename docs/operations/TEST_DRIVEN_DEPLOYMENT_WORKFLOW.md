# Test-Driven Deployment Workflow

**Perfect! You're absolutely right about the test-driven approach with alignment loops.**

## 🎯 **Enhanced Workflow with Test-Driven Alignment**

### **Phase 1: Discovery**
```bash
./scripts/ci/comprehensive-infrastructure-discovery.sh
```
**Purpose**: Complete server state analysis

### **Phase 2: Test-Align Loop** (Until PASS)
```bash
# TEST: Check deployment readiness
./scripts/ci/deployment-readiness-test.sh

# RESULT INTERPRETATION:
# Exit Code 0: ✅ PASS → Go to Phase 3 (Deploy)
# Exit Code 1: ❌ FAIL → Go to Alignment, then re-test
# Exit Code 2: ⚠️  CAUTION → 80%+ passed, decide policy
```

### **Alignment Sub-Phase** (If tests fail)
```bash
# Fix issues identified by readiness test
# Then re-run: ./scripts/ci/deployment-readiness-test.sh
# Loop until PASS
```

### **Phase 3: Deploy** (Only when tests PASS)
```bash
# Trigger GitHub Actions: self-hosted-deploy.yml
# OR run direct deployment
```

## 🧪 **New Test-Driven Scripts Created**

### **1. `deployment-readiness-test.sh`** - The Gatekeeper
**Purpose**: Tests server vs local working configuration alignment
**Tests**:
- ✅ Docker infrastructure (network, daemon)
- ✅ Monitoring stack (Prometheus, Grafana accessibility)
- ✅ Configuration files (volume paths, dashboard files)
- ✅ Service ports (available or responding)
- ✅ Data flow (Prometheus targets, Grafana datasources)

**Exit Codes**:
- `0`: All tests PASS → Deploy ready
- `1`: Critical failures → Must align
- `2`: Mostly PASS (80%+) → Caution/policy decision

### **2. `smart-deployment-orchestrator.sh`** - The Conductor
**Purpose**: Automated test-driven deployment workflow
**Features**:
- Discovery → Test → Align → Test loop
- Maximum 3 alignment attempts
- Automatic report generation
- Multiple report delivery options

## 📊 **Discovery Report Delivery Options**

### **Built-in Report Features**
```bash
# 1. File Report (Always)
deployment-reports/deployment-report-YYYYMMDD-HHMMSS.md

# 2. Email Report (Optional)
export DEPLOYMENT_REPORT_EMAIL="your-email@domain.com"

# 3. Webhook Report (Optional) - Slack/Discord/Teams
export DEPLOYMENT_REPORT_WEBHOOK="https://hooks.slack.com/your-webhook"

# 4. GitHub Integration (In CI)
# Automatic comments on deployment issues
```

### **Report Contents**
- Executive summary
- Discovery analysis (full output)
- Readiness test results (each attempt)
- Alignment actions taken
- Final deployment status
- Next steps/manual actions required

## 🔄 **Complete Workflow Examples**

### **Scenario 1: Perfect Alignment** (Skip Phase 2)
```bash
# Phase 1: Discovery
./scripts/ci/comprehensive-infrastructure-discovery.sh

# Phase 2: Test (PASS immediately)
./scripts/ci/deployment-readiness-test.sh
# ✅ Exit Code 0 - All tests passed

# Phase 3: Deploy
# Trigger GitHub Actions: self-hosted-deploy.yml
```

### **Scenario 2: Alignment Required** (Test-Align Loop)
```bash
# Phase 1: Discovery
./scripts/ci/comprehensive-infrastructure-discovery.sh

# Phase 2.1: Test (FAIL)
./scripts/ci/deployment-readiness-test.sh
# ❌ Exit Code 1 - Critical issues found

# Phase 2.1: Alignment
rsync -av infrastructure/ server:/opt/MEVana/infrastructure/
docker network create mevana_net

# Phase 2.2: Re-test
./scripts/ci/deployment-readiness-test.sh
# ✅ Exit Code 0 - Now ready

# Phase 3: Deploy
# Trigger deployment
```

### **Scenario 3: Automated Orchestration**
```bash
# Single command does everything
./scripts/ci/smart-deployment-orchestrator.sh

# This automatically:
# 1. Runs discovery
# 2. Tests readiness
# 3. Attempts alignment (up to 3 times)
# 4. Re-tests after each alignment
# 5. Deploys when ready OR reports blocking issues
# 6. Generates comprehensive report
```

## 🎯 **Answer to Your Questions**

### **Q1: If discovery shows alignment, skip Phase 2?**
**A**: **YES!** The `deployment-readiness-test.sh` will return exit code 0 if everything is aligned. You skip directly to Phase 3 (Deploy).

### **Q2: If alignment needed, go back to test?**
**A**: **YES!** The workflow is:
```
Discovery → Test → (FAIL?) → Align → Test → (FAIL?) → Align → Test → Deploy
```
Maximum 3 alignment attempts before requiring manual intervention.

### **Q3: Built-in tests to compare discovery vs working local?**
**A**: **YES!** The `deployment-readiness-test.sh` script compares:
- Volume paths (local working vs server)
- Dashboard files (arbitrage metrics presence)
- Configuration alignment (Prometheus jobs, etc.)
- Service accessibility (ports, endpoints)

## 📧 **Nifty Report Delivery**

### **Email Setup**
```bash
# Install mail command (if not present)
sudo apt-get install mailutils

# Set email for reports
export DEPLOYMENT_REPORT_EMAIL="your-email@domain.com"

# Run orchestrator - report automatically emailed
./scripts/ci/smart-deployment-orchestrator.sh
```

### **Slack/Discord Webhook**
```bash
# Set webhook URL
export DEPLOYMENT_REPORT_WEBHOOK="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"

# Run orchestrator - report posted to channel
./scripts/ci/smart-deployment-orchestrator.sh
```

### **GitHub Integration** (When running in Actions)
- Automatic issue comments on deployment problems
- PR status updates
- Deployment status badges

## 🚀 **Immediate Usage**

### **Manual Test-Driven Approach**
```bash
# 1. Discovery
ssh server "cd /opt/MEVana && ./scripts/ci/comprehensive-infrastructure-discovery.sh"

# 2. Test readiness
ssh server "cd /opt/MEVana && ./scripts/ci/deployment-readiness-test.sh"

# 3. If exit code 0 → Deploy via GitHub Actions
# 4. If exit code 1 → Fix alignment, re-test
```

### **Automated Approach**
```bash
# Single command orchestration
ssh server "cd /opt/MEVana && ./scripts/ci/smart-deployment-orchestrator.sh"

# With email reports
ssh server "cd /opt/MEVana && DEPLOYMENT_REPORT_EMAIL=your@email.com ./scripts/ci/smart-deployment-orchestrator.sh"
```

## 💪 **This Gives You**

✅ **Test-driven deployment** - No deployment without passing tests
✅ **Alignment loops** - Automatic retry until ready
✅ **Comprehensive reporting** - Know exactly what happened
✅ **Multiple delivery methods** - Email, Slack, GitHub, file
✅ **Safety gates** - Prevents bad deployments
✅ **Automated orchestration** - Single command full workflow

**Perfect deployment strategy with confidence and visibility!**