#!/bin/bash
# MEVana Latency Testing Toolkit for NY Datacenter Evaluation
# Run this from potential server locations to evaluate MEV trading performance

set -e

echo "🚀 MEVana Latency Testing Toolkit"
echo "=================================="
echo "Server Location: $(curl -s ipinfo.io/city 2>/dev/null || echo 'Unknown')"
echo "Public IP: $(curl -s ipinfo.io/ip 2>/dev/null || echo 'Unknown')"
echo "Timestamp: $(date)"
echo ""

# Create results directory
mkdir -p latency_results
RESULTS_FILE="latency_results/latency_test_$(date +%Y%m%d_%H%M%S).txt"

# Function to test latency with statistics
test_latency() {
    local host=$1
    local name=$2
    echo "Testing $name ($host)..."
    
    # Ping test (ICMP)
    echo "ICMP Ping:" >> $RESULTS_FILE
    ping -c 20 $host | tail -1 >> $RESULTS_FILE
    
    # TCP connection test (more realistic for API calls)
    echo "TCP Connection Test:" >> $RESULTS_FILE
    for i in {1..10}; do
        timeout 5 bash -c "time echo -e '\x1dclose\x0d' | telnet $host 443" 2>&1 | grep real | awk '{print $2}' >> temp_tcp.txt
    done
    
    if [ -f temp_tcp.txt ]; then
        echo "TCP Connect Times:" >> $RESULTS_FILE
        cat temp_tcp.txt >> $RESULTS_FILE
        rm temp_tcp.txt
    fi
    
    echo "---" >> $RESULTS_FILE
}

# Function to test HTTP API latency
test_api_latency() {
    local url=$1
    local name=$2
    echo "Testing API: $name ($url)..."
    
    echo "API Response Times for $name:" >> $RESULTS_FILE
    
    # Test with curl (most realistic for trading APIs)
    for i in {1..10}; do
        curl -w "@curl-format.txt" -o /dev/null -s "$url" 2>/dev/null >> temp_api.txt || echo "timeout" >> temp_api.txt
        sleep 0.1
    done
    
    if [ -f temp_api.txt ]; then
        cat temp_api.txt >> $RESULTS_FILE
        rm temp_api.txt
    fi
    
    echo "---" >> $RESULTS_FILE
}

# Create curl timing format file
cat > curl-format.txt << 'EOF'
     time_namelookup:  %{time_namelookup}\n
        time_connect:  %{time_connect}\n
     time_appconnect:  %{time_appconnect}\n
    time_pretransfer:  %{time_pretransfer}\n
       time_redirect:  %{time_redirect}\n
  time_starttransfer:  %{time_starttransfer}\n
                     ----------\n
          time_total:  %{time_total}\n
EOF

echo "🎯 Testing Critical MEV Trading Infrastructure..."
echo "================================================" >> $RESULTS_FILE

# Test Solana Infrastructure (Most Critical)
echo "📡 SOLANA INFRASTRUCTURE TESTS"
test_latency "api.mainnet-beta.solana.com" "Solana Mainnet RPC"
test_api_latency "https://api.mainnet-beta.solana.com" "Solana RPC API"

test_latency "rpc.shyft.to" "Shyft.to RPC (Our Geyser Provider)"
test_api_latency "https://rpc.shyft.to" "Shyft.to API"

# Test Major Solana Validators
echo ""
echo "🔗 SOLANA VALIDATOR NETWORK"
test_latency "entrypoint.mainnet-beta.solana.com" "Solana Entrypoint"
test_latency "mainnet-beta.solana.com" "Solana Mainnet"

# Test Major RPC Providers (Alternatives/Redundancy)
echo ""
echo "🌐 ALTERNATIVE RPC PROVIDERS"
test_latency "rpc.ankr.com" "Ankr RPC"
test_api_latency "https://rpc.ankr.com/solana" "Ankr Solana RPC"

test_latency "api.helius.xyz" "Helius RPC"
test_api_latency "https://api.helius.xyz" "Helius API"

test_latency "api.quicknode.com" "QuickNode RPC"

# Test DEX-specific Infrastructure
echo ""
echo "🔄 DEX INFRASTRUCTURE"
test_latency "api.raydium.io" "Raydium API"
test_api_latency "https://api.raydium.io" "Raydium DEX API"

# Test Jupiter (Future Integration)
test_latency "quote-api.jup.ag" "Jupiter Aggregator"
test_api_latency "https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111111111112&outputMint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v&amount=1000000" "Jupiter Quote API"

# Test Cloud Providers (For Comparison)
echo ""
echo "☁️ CLOUD PROVIDER COMPARISON"
test_latency "ec2.us-east-1.amazonaws.com" "AWS US-East-1"
test_latency "compute.googleapis.com" "Google Cloud"
test_latency "management.azure.com" "Microsoft Azure"

# Network Quality Tests
echo ""
echo "📊 NETWORK QUALITY ASSESSMENT"
echo "Network Quality Tests:" >> $RESULTS_FILE

# MTU Discovery
echo "MTU Size Test:" >> $RESULTS_FILE
ping -M do -s 1472 -c 1 api.mainnet-beta.solana.com >> $RESULTS_FILE 2>&1 || echo "Standard MTU failed" >> $RESULTS_FILE

# Bandwidth estimation (simple)
echo "Bandwidth Test (Simple):" >> $RESULTS_FILE
timeout 10 wget --progress=dot:giga -O /dev/null http://speedtest.tele2.net/10MB.zip 2>&1 | grep -o '[0-9.]\+ [KMG]B/s' | tail -1 >> $RESULTS_FILE

# Jitter test (critical for trading)
echo "Jitter Test (20 pings):" >> $RESULTS_FILE
ping -c 20 api.mainnet-beta.solana.com | grep 'min/avg/max/mdev' >> $RESULTS_FILE

echo ""
echo "✅ LATENCY TESTS COMPLETED!"
echo "Results saved to: $RESULTS_FILE"
echo ""

# Analyze results and provide summary
echo "📊 PERFORMANCE ANALYSIS"
echo "======================="

# Extract key metrics
SOLANA_AVG=$(grep "min/avg/max/mdev" $RESULTS_FILE | head -1 | awk -F'/' '{print $5}' | awk '{print $1}')
echo "Average Solana RPC Latency: ${SOLANA_AVG}ms"

# Grade the connection
if (( $(echo "$SOLANA_AVG < 5" | bc -l 2>/dev/null || echo 0) )); then
    echo "🟢 EXCELLENT: Premium datacenter quality (<5ms)"
    GRADE="A+"
elif (( $(echo "$SOLANA_AVG < 10" | bc -l 2>/dev/null || echo 0) )); then
    echo "🟡 VERY GOOD: Suitable for MEV trading (<10ms)"
    GRADE="A"
elif (( $(echo "$SOLANA_AVG < 20" | bc -l 2>/dev/null || echo 0) )); then
    echo "🟠 GOOD: Acceptable for arbitrage (<20ms)"
    GRADE="B"
else
    echo "🔴 POOR: Not optimal for competitive MEV (>20ms)"
    GRADE="C"
fi

echo ""
echo "📋 DATACENTER SUITABILITY REPORT"
echo "================================"
echo "Overall Grade: $GRADE"
echo "Solana Latency: ${SOLANA_AVG}ms"
echo "Recommended for MEV: $([ "$GRADE" = "A+" ] || [ "$GRADE" = "A" ] && echo "YES" || echo "EVALUATE ALTERNATIVES")"
echo ""
echo "💡 Compare this result across different datacenters!"
echo "Run this script from each potential server location."

# Cleanup
rm -f curl-format.txt

echo ""
echo "🔍 Next Steps:"
echo "1. Run this test from each potential datacenter"
echo "2. Compare results across locations"
echo "3. Consider network redundancy and failover"
echo "4. Test during peak trading hours (US market open)"
echo ""
echo "📁 Full results: $RESULTS_FILE"