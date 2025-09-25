#!/bin/bash

# =============================================================================
# MEVANA UI DEPLOYMENT SCRIPT - Bare Metal Deployment
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() { printf '\033[0;34m[INFO]\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m[WARN]\033[0m %s\n' "$1"; }
success() { printf '\033[0;32m[SUCCESS]\033[0m %s\n' "$1"; }
error() { printf '\033[0;31m[ERROR]\033[0m %s\n' "$1"; }

# Configuration
SERVER_HOST="${SERVER_HOST:-149.28.56.125}"
SERVER_USER="${SERVER_USER:-root}"
UI_PATH="${UI_PATH:-/opt/MEVana-UI}"
UI_REPO="${UI_REPO:-https://github.com/mike-inman/MEVana-UI.git}"
NODE_VERSION="${NODE_VERSION:-18}"
UI_PORT="${UI_PORT:-3200}"

log "🎨 MEVana UI Deployment Starting..."
log "Target: $SERVER_USER@$SERVER_HOST:$UI_PATH"
log "Port: $UI_PORT"

# =============================================================================
# PHASE 1: PREREQUISITES CHECK
# =============================================================================

log "📋 Phase 1: Checking prerequisites..."

check_prerequisites() {
    log "Checking Node.js $NODE_VERSION availability..."
    if ! command -v node >/dev/null 2>&1; then
        error "Node.js not found. Installing..."
        curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | sudo -E bash -
        sudo apt-get install -y nodejs
    fi

    local node_version=$(node --version | cut -d'v' -f2 | cut -d'.' -f1)
    if [ "$node_version" -lt "$NODE_VERSION" ]; then
        warn "Node.js version $node_version < $NODE_VERSION. Consider upgrading."
    fi

    log "Checking PM2 process manager..."
    if ! command -v pm2 >/dev/null 2>&1; then
        log "Installing PM2..."
        sudo npm install -g pm2
    fi

    success "Prerequisites verified"
}

# =============================================================================
# PHASE 2: DEPLOYMENT PREPARATION
# =============================================================================

log "🚀 Phase 2: Deployment preparation..."

prepare_deployment() {
    log "Creating deployment directory..."
    sudo mkdir -p "$UI_PATH"
    sudo mkdir -p /etc/mevana

    log "Setting up mevana user permissions..."
    if ! id "mevana" >/dev/null 2>&1; then
        sudo useradd -r -s /bin/false mevana
    fi
    sudo chown -R mevana:mevana "$UI_PATH"

    success "Deployment preparation complete"
}

# =============================================================================
# PHASE 3: CODE DEPLOYMENT
# =============================================================================

log "📦 Phase 3: Code deployment..."

deploy_code() {
    log "Cloning/updating MEVana-UI repository..."
    if [ -d "$UI_PATH/.git" ]; then
        cd "$UI_PATH"
        sudo -u mevana git pull origin main
    else
        sudo -u mevana git clone "$UI_REPO" "$UI_PATH"
        cd "$UI_PATH"
    fi

    log "Installing Node.js dependencies..."
    sudo -u mevana npm ci --only=production

    log "Building production bundle..."
    sudo -u mevana npm run build

    success "Code deployment complete"
}

# =============================================================================
# PHASE 4: CONFIGURATION
# =============================================================================

log "⚙️ Phase 4: Configuration..."

configure_environment() {
    log "Creating production environment file..."
    cat > /tmp/ui.env << EOF
# Production Environment for MEVana UI
NODE_ENV=production
PORT=$UI_PORT

# MEVana Core Services (Production URLs)
NEXT_PUBLIC_NORMALIZER_URL=http://$SERVER_HOST:9901
NEXT_PUBLIC_EDGE_ASSEMBLER_URL=http://$SERVER_HOST:9902
NEXT_PUBLIC_ARB_ENGINE_URL=http://$SERVER_HOST:9903
NEXT_PUBLIC_EXECUTOR_SERVICE_URL=http://$SERVER_HOST:9904
NEXT_PUBLIC_DISCOVERY_SERVICE_URL=http://$SERVER_HOST:9102
NEXT_PUBLIC_TOKEN_REGISTRY_URL=http://$SERVER_HOST:9898
NEXT_PUBLIC_HOTPATH_ARB_ENGINE_URL=http://$SERVER_HOST:8088
NEXT_PUBLIC_MONITORING_SERVICE_URL=http://$SERVER_HOST:8091
NEXT_PUBLIC_WALLET_VAULT_URL=http://$SERVER_HOST:8090

# Clerk Authentication (Production)
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=\${CLERK_PUBLISHABLE_KEY:-}
CLERK_SECRET_KEY=\${CLERK_SECRET_KEY:-}
NEXT_PUBLIC_CLERK_SIGN_IN_URL=/sign-in
NEXT_PUBLIC_CLERK_SIGN_UP_URL=/sign-up
NEXT_PUBLIC_CLERK_FALLBACK_REDIRECT_URL=/dashboard

# Executor Admin API
EXECUTOR_ADMIN_URL=http://$SERVER_HOST:9898
CLERK_EXECUTOR_JWT_TEMPLATE=executor-admin
EOF

    sudo mv /tmp/ui.env /etc/mevana/ui.env
    sudo chown root:mevana /etc/mevana/ui.env
    sudo chmod 640 /etc/mevana/ui.env

    success "Environment configuration complete"
}

# =============================================================================
# PHASE 5: SYSTEMD SERVICE SETUP
# =============================================================================

log "🔧 Phase 5: Systemd service setup..."

setup_systemd() {
    log "Creating systemd service file..."
    cat > /tmp/mevana-ui.service << EOF
[Unit]
Description=MEVana UI Dashboard
After=network.target
Requires=network.target

[Service]
Type=simple
User=mevana
WorkingDirectory=$UI_PATH
EnvironmentFile=/etc/mevana/ui.env
ExecStart=/usr/bin/npm start
Restart=always
RestartSec=5
StartLimitInterval=60
StartLimitBurst=3

# Security settings
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$UI_PATH
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

    sudo mv /tmp/mevana-ui.service /etc/systemd/system/mevana-ui.service
    sudo systemctl daemon-reload
    sudo systemctl enable mevana-ui.service

    success "Systemd service configured"
}

# =============================================================================
# PHASE 6: SERVICE STARTUP & VALIDATION
# =============================================================================

log "▶️ Phase 6: Service startup and validation..."

start_and_validate() {
    log "Starting MEVana UI service..."
    sudo systemctl start mevana-ui.service

    log "Waiting for service to initialize..."
    sleep 10

    log "Checking service status..."
    if sudo systemctl is-active --quiet mevana-ui.service; then
        success "MEVana UI service is running"
    else
        error "MEVana UI service failed to start"
        sudo systemctl status mevana-ui.service
        sudo journalctl -u mevana-ui.service -n 20
        exit 1
    fi

    log "Testing HTTP endpoint..."
    if curl -f -s "http://localhost:$UI_PORT" >/dev/null; then
        success "MEVana UI is responding on port $UI_PORT"
    else
        warn "MEVana UI may still be initializing..."
        log "Waiting additional 30 seconds..."
        sleep 30
        if curl -f -s "http://localhost:$UI_PORT" >/dev/null; then
            success "MEVana UI is now responding"
        else
            error "MEVana UI is not responding after 40 seconds"
            exit 1
        fi
    fi
}

# =============================================================================
# MAIN EXECUTION
# =============================================================================

main() {
    check_prerequisites
    prepare_deployment
    deploy_code
    configure_environment
    setup_systemd
    start_and_validate

    success "🎉 MEVana UI Deployment Complete!"
    log ""
    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log "🌐 UI Access: http://$SERVER_HOST:$UI_PORT"
    log "📊 Service Status: sudo systemctl status mevana-ui"
    log "📜 Logs: sudo journalctl -u mevana-ui -f"
    log "🔄 Restart: sudo systemctl restart mevana-ui"
    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Run deployment
main "$@"