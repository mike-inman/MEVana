#!/bin/bash

# Shyft API Testing Script
# Created: 2024-08-19 15:30 UTC
# Purpose: Test Shyft APIs for hybrid strategy implementation

echo "Starting Shyft API Testing Suite..."
echo "API Key: fWrVh_mDygK358FC"
echo "Network: mainnet-beta"
echo ""

# Test 1: DeFi Pool Discovery API - SOL Token
echo "Testing DeFi Pool Discovery API (SOL)..."
curl -H "x-api-key: fWrVh_mDygK358FC" \
     "https://defi.shyft.to/v0/pools/get_all_pools_for_token?token=So11111111111111111111111111111111111111112"
echo ""
echo "----------------------------------------"
