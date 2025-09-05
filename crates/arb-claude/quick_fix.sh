#!/bin/bash

# Quick Arbitrage Engine Diagnosis & Fix Script
set -e

echo "🔍 Arbitrage Engine Quick Diagnosis"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Check if arb-engine is running
echo -e "\n1️⃣ Checking if arb-engine is running..."
if pgrep -f "arb-engine" > /dev/null; then
    echo -e "${GREEN}✅ Arb-engine is running${NC}"
    
    # Check health
    if curl -s http://localhost:8080/ready > /dev/null 2>&1; then