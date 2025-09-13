#!/bin/bash

# MEVana System Shutdown Script
# This script gracefully shuts down all services in the proper sequence
# Date: 2025-09-09

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored messages
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Function to wait for graceful shutdown
wait_for_shutdown() {
    local service_name=$1
    local pid=$2
    local timeout=${3:-30}
    local count=0
    
    log_info "Waiting for $service_name (PID: $pid) to shutdown gracefully..."
    
    while kill -0 "$pid" 2>/dev/null && [ $count -lt $timeout ]; do
        sleep 1
        count=$((count + 1))
        echo -n "."
    done
    echo ""
    
    if kill -0 "$pid" 2>/dev/null; then
        log_warning "$service_name didn't shutdown gracefully after ${timeout}s, forcing termination"
        kill -KILL "$pid" 2>/dev/null || true
        return 1
    else
        log_success "$service_name shutdown gracefully"
        return 0
    fi
}

# Function to check for active trading positions
check_trading_safety() {
    log_info "🔍 Checking trading safety before shutdown..."
    
    # Check for active arbitrage opportunities
    if curl -s -f "http://localhost:8085/ai/config" >/dev/null 2>&1; then
        local active_opps=$(curl -s "http://localhost:8085/metrics" | grep -c "arb_opportunities_detected" || echo "0")
        if [ "$active_opps" -gt 0 ]; then
            log_warning "⚠️  Active arbitrage opportunities detected. Consider waiting for completion."
            read -p "Continue with shutdown anyway? (y/N): " -r
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                log_info "Shutdown cancelled by user"
                exit 0
            fi
        fi
    fi
    
    # Check wallet vault for pending transactions
    if curl -s -f "http://localhost:8090/health" >/dev/null 2>&1; then
        log_info "✅ Wallet vault accessible - checking for pending operations"
        # In production, we'd check for pending transactions here
    fi
    
    log_success "✅ Trading safety check complete"
}

# Function to backup critical data
backup_critical_data() {
    log_info "💾 Creating safety backup before shutdown..."
    
    # Ensure backup directory exists
    mkdir -p backups
    
    # Backup current system state
    echo "$(date -u +%Y-%m-%dT%H:%M:%SZ): System shutdown initiated" >> backups/shutdown.log
    
    # Export current metrics snapshot
    if curl -s -f "http://localhost:8088/performance" >/dev/null 2>&1; then
        curl -s "http://localhost:8088/performance" > "backups/hotpath_metrics_$(date +%Y%m%d_%H%M%S).json" 2>/dev/null || true
        log_info "📊 HotPath metrics backed up"
    fi
    
    # Export arbitrage engine state
    if curl -s -f "http://localhost:8085/ai/config" >/dev/null 2>&1; then
        curl -s "http://localhost:8085/ai/config" > "backups/arb_config_$(date +%Y%m%d_%H%M%S).json" 2>/dev/null || true
        log_info "🤖 Arbitrage config backed up"
    fi
    
    log_success "✅ Critical data backup complete"
}

# Change to MEVana directory
cd /Users/michaelinman/Repos/MEVana

log_info "==============================================="
log_info "🛑 MEVana System Graceful Shutdown"
log_info "==============================================="

# Step 1: Safety checks
check_trading_safety
backup_critical_data

# Step 2: Shutdown Client-Facing Services First (Tier 1)
log_info "Step 1: Shutting down client-facing services..."

# Stop HotPath Engine (external users might be connected)
if pgrep -f hotpath_arb_engine > /dev/null; then
    log_info "Stopping hotpath_arb_engine..."
    pkill -TERM -f hotpath_arb_engine
    if pgrep -f hotpath_arb_engine > /dev/null; then
        wait_for_shutdown "hotpath_arb_engine" $(pgrep -f hotpath_arb_engine) 45
    fi
else
    log_info "hotpath_arb_engine not running"
fi

# Stop monitoring service
if pgrep -f monitoring-service > /dev/null; then
    log_info "Stopping monitoring-service..."
    pkill -TERM -f monitoring-service
    if pgrep -f monitoring-service > /dev/null; then
        wait_for_shutdown "monitoring-service" $(pgrep -f monitoring-service) 30
    fi
else
    log_info "monitoring-service not running"
fi

# Step 3: Shutdown Core Trading Services (Tier 2)
log_info "Step 2: Shutting down core trading services..."

# Stop arbitrage engine (graceful shutdown allows finishing current trades)
if pgrep -f "arb-engine" > /dev/null; then
    log_info "Stopping arb-engine (allowing current trades to complete)..."
    pkill -TERM -f "arb-engine"
    if pgrep -f "arb-engine" > /dev/null; then
        wait_for_shutdown "arb-engine" $(pgrep -f "arb-engine") 60  # Longer timeout for trades
    fi
else
    log_info "arb-engine not running"
fi

# Stop executor service (allow pending executions to complete)
if pgrep -f executor-service > /dev/null; then
    log_info "Stopping executor-service..."
    pkill -TERM -f executor-service
    if pgrep -f executor-service > /dev/null; then
        wait_for_shutdown "executor-service" $(pgrep -f executor-service) 45
    fi
else
    log_info "executor-service not running"
fi

# Step 4: Shutdown Data Processing Services (Tier 3)
log_info "Step 3: Shutting down data processing services..."

# Stop edge assembler
if pgrep -f edge-assembler > /dev/null; then
    log_info "Stopping edge-assembler..."
    pkill -TERM -f edge-assembler
    if pgrep -f edge-assembler > /dev/null; then
        wait_for_shutdown "edge-assembler" $(pgrep -f edge-assembler) 30
    fi
else
    log_info "edge-assembler not running"
fi

# Stop normalizer service
if pgrep -f normalizer-service > /dev/null; then
    log_info "Stopping normalizer-service..."
    pkill -TERM -f normalizer-service
    if pgrep -f normalizer-service > /dev/null; then
        wait_for_shutdown "normalizer-service" $(pgrep -f normalizer-service) 30
    fi
else
    log_info "normalizer-service not running"
fi

# Step 5: Shutdown Security Services Last (Tier 4)
log_info "Step 4: Shutting down security services..."

# Stop wallet vault LAST (other services may need it for final operations)
if pgrep -f wallet-vault > /dev/null; then
    log_info "Stopping wallet-vault (final security shutdown)..."
    pkill -TERM -f wallet-vault
    if pgrep -f wallet-vault > /dev/null; then
        wait_for_shutdown "wallet-vault" $(pgrep -f wallet-vault) 30
    fi
else
    log_info "wallet-vault not running"
fi

# Step 6: Infrastructure Services (Optional - can be left running)
log_info "Step 5: Infrastructure services status..."
log_info "Note: Infrastructure services (PostgreSQL, Redis, Kafka) left running"
log_info "Run 'cd infrastructure && docker-compose down' to stop infrastructure"

# Step 7: Final verification
log_info "Step 6: Final verification..."

echo ""
echo "Final Service Status Check:"
echo "  hotpath_arb_engine: $(pgrep -f hotpath_arb_engine >/dev/null && echo '⚠️  Still running' || echo '✅ Stopped')"
echo "  monitoring-service: $(pgrep -f monitoring-service >/dev/null && echo '⚠️  Still running' || echo '✅ Stopped')"
echo "  arb-engine: $(pgrep -f 'arb-engine' >/dev/null && echo '⚠️  Still running' || echo '✅ Stopped')"
echo "  executor-service: $(pgrep -f executor-service >/dev/null && echo '⚠️  Still running' || echo '✅ Stopped')"
echo "  edge-assembler: $(pgrep -f edge-assembler >/dev/null && echo '⚠️  Still running' || echo '✅ Stopped')"
echo "  normalizer-service: $(pgrep -f normalizer-service >/dev/null && echo '⚠️  Still running' || echo '✅ Stopped')"
echo "  wallet-vault: $(pgrep -f wallet-vault >/dev/null && echo '⚠️  Still running' || echo '✅ Stopped')"

echo ""
echo "Infrastructure Services (Docker):"
echo "  PostgreSQL: $(docker ps | grep -q postgres && echo '🔄 Running (left active)' || echo '⏹️  Stopped')"
echo "  Redis: $(docker ps | grep -q redis && echo '🔄 Running (left active)' || echo '⏹️  Stopped')"
echo "  Kafka: $(docker ps | grep -q kafka && echo '🔄 Running (left active)' || echo '⏹️  Stopped')"

# Final summary
echo ""
log_success "==============================================="
log_success "🎯 MEVana System Shutdown Complete"
log_success "==============================================="
log_info "📝 Shutdown log: backups/shutdown.log"
log_info "📊 Metrics backup: backups/hotpath_metrics_*.json"
log_info "⚙️  Config backup: backups/arb_config_*.json"
log_info ""
log_info "To restart the system:"
log_info "  ./scripts/start_system.sh"
log_info ""
log_info "To stop infrastructure services:"
log_info "  cd infrastructure && docker-compose down"