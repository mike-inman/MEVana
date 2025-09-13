#!/bin/bash

# =============================================================================
# MEVANA SERVER UPDATE SCRIPT
# =============================================================================

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1"
}

# Get server IP from user
echo "Enter your Vultr server IP address:"
read SERVER_IP

if [[ -z "$SERVER_IP" ]]; then
    echo "Error: Server IP is required"
    exit 1
fi

SERVER_USER="root"
SERVER_DIR="/opt/MEVana"

log "🚀 Updating MEVana on server: $SERVER_IP"

# =============================================================================
# PHASE 1: Commit and Push Changes
# =============================================================================

log "📝 Committing latest changes..."

# Add all changes
git add .

# Commit with timestamp
COMMIT_MSG="feat: streamlined deployment with all venues + Jupiter integration

- Fixed Docker protobuf-compiler dependency
- Added deploy_fast.sh with multi-venue support
- Enabled Jupiter multi-hop routing
- Optimized for weekend trading deployment

🤖 Generated with Claude Code"

git commit -m "$COMMIT_MSG" || log "No new changes to commit"

# Push to main
git push origin main

log "✅ Changes pushed to repository"

# =============================================================================
# PHASE 2: Update Server Code
# =============================================================================

log "🔄 Updating server code..."

ssh $SERVER_USER@$SERVER_IP << 'EOF'
cd /opt/MEVana

# Pull latest changes
git pull origin main

# Install any missing dependencies
apt-get update
apt-get install -y bc curl protobuf-compiler docker.io docker-compose

# Create Docker network if it doesn't exist
if ! docker network ls | grep -q "mevana_net"; then
    echo "Creating Docker network..."
    docker network create mevana_net
else
    echo "Docker network already exists"
fi

# Create logs directory
mkdir -p logs

# Stop any running services to avoid port conflicts
pkill -f "arb-engine" || true
pkill -f "whirlpool-live-stream" || true
sleep 2

echo "✅ Server code updated and network ready"
EOF

# =============================================================================
# PHASE 3: Deploy with Fast Script
# =============================================================================

log "🚀 Running fast deployment on server..."

ssh $SERVER_USER@$SERVER_IP << 'EOF'
cd /opt/MEVana

# Make script executable
chmod +x deploy_fast.sh

# Run the deployment
./deploy_fast.sh

echo "✅ Fast deployment completed"
EOF

# =============================================================================
# PHASE 4: Test Connectivity and Performance
# =============================================================================

log "🔍 Testing server performance..."

# Test latency to server
log "   • Testing SSH connectivity..."
START_TIME=$(date +%s%N)
ssh $SERVER_USER@$SERVER_IP 'echo "SSH OK"' >/dev/null
END_TIME=$(date +%s%N)
SSH_LATENCY_MS=$(echo "scale=3; ($END_TIME - $START_TIME) / 1000000" | bc -l)
log "   • SSH latency: ${SSH_LATENCY_MS}ms"

# Test arbitrage engine API
log "   • Testing arbitrage engine API..."
if ssh $SERVER_USER@$SERVER_IP 'curl -s http://localhost:8085/ai/config' >/dev/null 2>&1; then
    log "✅ Arbitrage engine API is responding"
else
    warn "Arbitrage engine API not yet ready (may take a few minutes)"
fi

# =============================================================================
# PHASE 5: Show Status and Commands
# =============================================================================

log "📊 SERVER DEPLOYMENT COMPLETE!"
echo
log "🌐 Server Access:"
log "   • SSH:           ssh $SERVER_USER@$SERVER_IP"
log "   • Kafka UI:      http://$SERVER_IP:8080"
log "   • Grafana:       http://$SERVER_IP:3000"
log "   • Arbitrage API: http://$SERVER_IP:8085/ai/config"

echo
log "📈 Live Trading Status:"
ssh $SERVER_USER@$SERVER_IP << 'EOF'
cd /opt/MEVana

echo "   • Arbitrage Engine Status:"
if pgrep -f "arb-engine" >/dev/null; then
    echo "     ✅ Running (PID: $(pgrep -f arb-engine))"
else
    echo "     ⏳ Starting up..."
fi

echo "   • Discovery Stream Status:"
if pgrep -f "whirlpool-live-stream" >/dev/null; then
    echo "     ✅ Running (PID: $(pgrep -f whirlpool-live-stream))"
else
    echo "     ⏳ Starting up..."
fi

# Show recent activity
echo
echo "   • Recent Activity (last 10 lines):"
if [[ -f logs/arb-engine.log ]]; then
    tail -n 10 logs/arb-engine.log | grep -E "(EDGE_STATS|INFO|WARN)" | tail -n 5 || echo "     Waiting for activity..."
else
    echo "     Log file not yet created..."
fi
EOF

echo
log "🔧 Useful Commands:"
log "   • Monitor live:     ssh $SERVER_USER@$SERVER_IP 'tail -f /opt/MEVana/logs/arb-engine.log'"
log "   • Check status:     ssh $SERVER_USER@$SERVER_IP 'pgrep -f arb-engine'"
log "   • Restart engine:   ssh $SERVER_USER@$SERVER_IP 'cd /opt/MEVana && ./restart_engine.sh'"
log "   • View metrics:     curl http://$SERVER_IP:8085/metrics"

echo
log "✅ MEVana server update complete - Ready for live trading!"