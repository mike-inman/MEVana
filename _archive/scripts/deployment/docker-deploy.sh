#!/bin/bash
# Deprecated: Use GHCR-based deploy and compose overlays (see scripts/deployment)
# Last updated: 2025-09-15

# MEVana Full Docker Deployment - EVERYTHING in containers
# Usage: ./scripts/docker-deploy.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

log_info "🚀 MEVana Full Docker Deployment Started"

# Step 1: Kill all background processes
log_info "Step 1: Cleaning up background processes"
pkill -f "hotpath_arb_engine" || true
pkill -f "cargo run" || true
sleep 2
log_success "Background processes cleaned"

# Step 2: Build application image
log_info "Step 2: Building application Docker image"
docker build -t mevana/hotpath_arb_engine:latest -f Dockerfile .
log_success "Application image built"

# Step 3: Deploy everything via Docker Compose
log_info "Step 3: Deploying full stack via Docker Compose"
cd infrastructure

# Create network if it doesn't exist
docker network create mevana_net 2>/dev/null || true

# Deploy infrastructure + application
docker compose up -d

log_success "Full stack deployed in Docker"

# Step 4: Wait and verify
log_info "Step 4: Waiting for services to start..."
sleep 30

# Check containers
log_info "Container status:"
docker compose ps

# Step 5: Data verification
log_info "Step 5: Verifying data endpoints"

# Check hotpath_arb_engine metrics
if curl -s http://localhost:8088/metrics | grep -q "arbitrage"; then
    log_success "HotPath engine metrics verified - arbitrage data flowing"
else
    log_error "HotPath engine metrics not found"
fi

# Check Prometheus
if curl -s http://localhost:9090/-/healthy >/dev/null; then
    log_success "Prometheus healthy"
else
    log_error "Prometheus not healthy"
fi

# Check Grafana
if curl -s http://localhost:3000/api/health >/dev/null; then
    log_success "Grafana healthy"
else
    log_error "Grafana not healthy"
fi

log_success "🎉 MEVana Full Docker Deployment Complete!"
log_info "📊 Access points:"
log_info "   • HotPath Engine: http://localhost:8088/metrics"
log_info "   • Grafana: http://localhost:3000 (admin/<ADMIN_PASSWORD>)"
log_info "   • Prometheus: http://localhost:9090"
log_info "   • Kafka UI: http://localhost:8080"
log_info "   • PostgreSQL: localhost:5433 (mev/<POSTGRES_PASSWORD>)"
