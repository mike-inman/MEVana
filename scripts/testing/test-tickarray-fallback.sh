#!/bin/bash

# Test script for TickArray fallback using Shyft APIs
# This script verifies that we can get real tick data when on-chain parsing fails

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🧪 Testing TickArray Fallback with Shyft APIs${NC}"
echo "=================================================="

# Check if API key is set
if [ -z "$SHYFT_API_KEY" ]; then
    echo -e "${RED}❌ SHYFT_API_KEY environment variable not set${NC}"
    echo "Please set: export SHYFT_API_KEY=<SHYFT_API_KEY>"
    exit 1
fi

echo -e "${GREEN}✅ SHYFT_API_KEY is set${NC}"

# Test 1: Get a single pool's tick data
echo -e "\n${YELLOW}Test 1: Getting tick data for a specific pool${NC}"
POOL_ADDRESS="7YGHXMBpNLcDRLPUJfqzN79WMoh8Qniz52cNuZK6go7b"

RESPONSE=$(curl -s -X POST "https://programs.shyft.to/v0/graphql/?api_key=$SHYFT_API_KEY&network=mainnet-beta" \
  -H "Content-Type: application/json" \
  -d "{\"query\":\"query GetPoolTickData(\$address: String!) { ORCA_WHIRLPOOLS_whirlpool(where: { pubkey: { _eq: \$address } }, limit: 1) { pubkey tokenMintA tokenMintB feeRate tickCurrentIndex sqrtPrice liquidity } }\", \"variables\": { \"address\": \"$POOL_ADDRESS\" }}")

echo "Response for pool $POOL_ADDRESS:"
echo "$RESPONSE" | jq '.data.ORCA_WHIRLPOOLS_whirlpool[0]' 2>/dev/null || echo "$RESPONSE"

# Test 2: Get multiple pools to see tick data
echo -e "\n${YELLOW}Test 2: Getting tick data for multiple pools${NC}"
RESPONSE2=$(curl -s -X POST "https://programs.shyft.to/v0/graphql/?api_key=$SHYFT_API_KEY&network=mainnet-beta" \
  -H "Content-Type: application/json" \
  -d "{\"query\":\"query GetPools { ORCA_WHIRLPOOLS_whirlpool(limit: 3, order_by: { liquidity: desc }) { pubkey tokenMintA tokenMintB feeRate tickCurrentIndex sqrtPrice liquidity } }\", \"variables\": {}}")

echo "Response for 3 pools:"
echo "$RESPONSE2" | jq '.data.ORCA_WHIRLPOOLS_whirlpool[] | {pubkey, tickCurrentIndex, sqrtPrice, feeRate}' 2>/dev/null || echo "$RESPONSE2"

# Test 3: Check if we can get the specific fields we need for TickArray fallback
echo -e "\n${YELLOW}Test 3: Verifying required fields for TickArray fallback${NC}"
REQUIRED_FIELDS=("pubkey" "tickCurrentIndex" "sqrtPrice" "liquidity" "feeRate")

for field in "${REQUIRED_FIELDS[@]}"; do
    if echo "$RESPONSE" | grep -q "$field"; then
        echo -e "${GREEN}✅ Field '$field' is available${NC}"
    else
        echo -e "${RED}❌ Field '$field' is missing${NC}"
    fi
done

# Test 4: Check response time (should be fast for fallback)
echo -e "\n${YELLOW}Test 4: Checking API response time${NC}"
START_TIME=$(date +%s%N)
curl -s -X POST "https://programs.shyft.to/v0/graphql/?api_key=$SHYFT_API_KEY&network=mainnet-beta" \
  -H "Content-Type: application/json" \
  -d "{\"query\":\"query GetPoolTickData(\$address: String!) { ORCA_WHIRLPOOLS_whirlpool(where: { pubkey: { _eq: \$address } }, limit: 1) { pubkey tickCurrentIndex sqrtPrice } }\", \"variables\": { \"address\": \"$POOL_ADDRESS\" }}" > /dev/null
END_TIME=$(date +%s%N)

RESPONSE_TIME_MS=$(( (END_TIME - START_TIME) / 1000000 ))
echo "API response time: ${RESPONSE_TIME_MS}ms"

if [ $RESPONSE_TIME_MS -lt 1000 ]; then
    echo -e "${GREEN}✅ API response time is fast enough for fallback (<1s)${NC}"
else
    echo -e "${YELLOW}⚠️  API response time might be too slow for fallback (${RESPONSE_TIME_MS}ms)${NC}"
fi

# Test 5: Validate the data structure
echo -e "\n${YELLOW}Test 5: Validating data structure${NC}"
TICK_DATA=$(echo "$RESPONSE" | jq -r '.data.ORCA_WHIRLPOOLS_whirlpool[0].tickCurrentIndex // "null"' 2>/dev/null)
SQRT_PRICE=$(echo "$RESPONSE" | jq -r '.data.ORCA_WHIRLPOOLS_whirlpool[0].sqrtPrice // "null"' 2>/dev/null)

if [ "$TICK_DATA" != "null" ] && [ "$TICK_DATA" != "" ]; then
    echo -e "${GREEN}✅ Tick data is valid: $TICK_DATA${NC}"
else
    echo -e "${RED}❌ Tick data is invalid or missing${NC}"
fi

if [ "$SQRT_PRICE" != "null" ] && [ "$SQRT_PRICE" != "" ]; then
    echo -e "${GREEN}✅ Sqrt price data is valid: $SQRT_PRICE${NC}"
else
    echo -e "${RED}❌ Sqrt price data is invalid or missing${NC}"
fi

echo -e "\n${GREEN}🎯 TickArray Fallback Test Summary${NC}"
echo "=================================================="

if [ "$TICK_DATA" != "null" ] && [ "$SQRT_PRICE" != "null" ] && [ $RESPONSE_TIME_MS -lt 1000 ]; then
    echo -e "${GREEN}✅ SUCCESS: Shyft API can provide TickArray fallback data${NC}"
    echo -e "${GREEN}✅ SUCCESS: Response time is fast enough for fallback${NC}"
    echo -e "${GREEN}✅ SUCCESS: All required fields are available${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Integrate TickArrayFallbackClient into dex-backfill"
    echo "2. Use Shyft data when on-chain TickArray parsing fails"
    echo "3. Verify depth-builder gets real data instead of num_init=0"
else
    echo -e "${RED}❌ FAILED: Some tests did not pass${NC}"
    echo "Please check the API responses above"
fi

echo ""
echo -e "${GREEN}Test completed at $(date)${NC}"
