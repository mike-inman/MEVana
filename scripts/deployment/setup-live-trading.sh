#!/bin/bash
# MEVana Live Trading Setup Script
# Prepares everything needed for one-click live trading activation

set -e

echo "🚀 MEVana Live Trading Setup"
echo "============================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check prerequisites
echo ""
echo "🔍 Checking Prerequisites..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo is not installed. Please install from https://rustup.rs/"
    exit 1
fi
print_status "Rust/Cargo is installed"

# Check if Solana CLI is installed
if ! command -v solana &> /dev/null; then
    print_warning "Solana CLI not found. Installing..."
    sh -c "$(curl -sSfL https://release.solana.com/v1.18.4/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi
print_status "Solana CLI is available"

# Check if PostgreSQL is running
if ! pg_isready -h localhost -p 5433 &> /dev/null; then
    print_warning "PostgreSQL not running on port 5433. Please start with: docker-compose up -d"
fi

# Check if Redis is running  
if ! redis-cli -p 6379 ping &> /dev/null; then
    print_warning "Redis not running on port 6379. Please start with: docker-compose up -d"
fi

echo ""
echo "⚙️  Setting Up Live Trading Infrastructure..."

# 1. Create environment configuration
if [ ! -f ".env" ]; then
    print_info "Creating .env configuration file..."
    cp .env.live-trading-template .env
    print_status "Created .env from template"
    print_warning "IMPORTANT: Edit .env file to customize your settings!"
else
    print_warning ".env file already exists. Please review settings."
fi

# 2. Generate Jito auth keypair if not exists
JITO_KEYPAIR_PATH="./jito-auth-keypair.json"
if [ ! -f "$JITO_KEYPAIR_PATH" ]; then
    print_info "Generating Jito authentication keypair..."
    solana-keygen new --no-bip39-passphrase -o "$JITO_KEYPAIR_PATH"
    print_status "Jito keypair generated: $JITO_KEYPAIR_PATH"
    print_warning "Keep this keypair secure - it's needed for Jito bundle submission!"
else
    print_status "Jito keypair already exists: $JITO_KEYPAIR_PATH"
fi

# 3. Generate trading wallets
WALLETS_DIR="./trading-wallets"
if [ ! -d "$WALLETS_DIR" ]; then
    print_info "Creating trading wallet directory..."
    mkdir -p "$WALLETS_DIR"
    
    # Generate hot wallets
    for i in {1..2}; do
        solana-keygen new --no-bip39-passphrase -o "$WALLETS_DIR/hot-wallet-$i.json"
        PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/hot-wallet-$i.json")
        echo "Hot Wallet $i: $PUBKEY" >> "$WALLETS_DIR/wallet-addresses.txt"
    done
    
    # Generate warm wallet
    solana-keygen new --no-bip39-passphrase -o "$WALLETS_DIR/warm-wallet.json"
    WARM_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/warm-wallet.json")
    echo "Warm Wallet: $WARM_PUBKEY" >> "$WALLETS_DIR/wallet-addresses.txt"
    
    # Generate cold wallet
    solana-keygen new --no-bip39-passphrase -o "$WALLETS_DIR/cold-wallet.json"
    COLD_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/cold-wallet.json")
    echo "Cold Wallet: $COLD_PUBKEY" >> "$WALLETS_DIR/wallet-addresses.txt"
    
    print_status "Trading wallets generated in $WALLETS_DIR/"
    print_warning "FUND THESE WALLETS WITH SMALL AMOUNTS BEFORE GOING LIVE!"
    print_info "Wallet addresses saved to: $WALLETS_DIR/wallet-addresses.txt"
else
    print_status "Trading wallets directory already exists"
fi

# 4. Build the project with all live trading features
echo ""
print_info "Building MEVana with live trading features..."
cargo build --release --features "live-trading,jito-integration"

if [ $? -eq 0 ]; then
    print_status "Build completed successfully"
else
    print_error "Build failed. Please fix compilation errors."
    exit 1
fi

# 5. Create startup scripts
print_info "Creating startup scripts..."

# Dry-run startup script
cat > start-dry-run.sh << 'EOF'
#!/bin/bash
# Start MEVana in dry-run mode for testing

echo "🧪 Starting MEVana in DRY-RUN mode..."
echo "This is safe for testing - no real trades will be executed"

export ENABLE_LIVE_EXECUTION=false
export DRY_RUN=true  
export SIMULATION_MODE=true
export RUST_LOG=info

./target/release/hotpath_arb_engine
EOF
chmod +x start-dry-run.sh

# Live trading startup script  
cat > start-live-trading.sh << 'EOF'
#!/bin/bash
# Start MEVana in LIVE TRADING mode
# ⚠️  WARNING: This will execute real trades with real money!

echo "🚨 STARTING MEVANA IN LIVE TRADING MODE"
echo "======================================="
echo "⚠️  WARNING: This will execute REAL trades with REAL money!"
echo "⚠️  Make sure you have:"
echo "   - Funded your wallets with appropriate amounts"
echo "   - Tested thoroughly in dry-run mode first"
echo "   - Configured all safety settings properly"
echo ""
read -p "Are you absolutely sure you want to enable live trading? (type 'YES' to continue): " confirm

if [ "$confirm" != "YES" ]; then
    echo "❌ Live trading cancelled by user"
    exit 1
fi

echo "🚀 Starting live trading in 5 seconds..."
echo "Press Ctrl+C to cancel now!"
sleep 5

export ENABLE_LIVE_EXECUTION=true
export DRY_RUN=false
export SIMULATION_MODE=false
export RUST_LOG=info

./target/release/hotpath_arb_engine
EOF
chmod +x start-live-trading.sh

print_status "Created startup scripts:"
print_info "  - start-dry-run.sh (safe testing)"
print_info "  - start-live-trading.sh (REAL MONEY trading)"

# 6. Create monitoring script
cat > monitor-trading.sh << 'EOF'
#!/bin/bash
# Monitor live trading performance

echo "📊 MEVana Trading Monitor"
echo "========================"

# Check if process is running
if pgrep -f "hotpath_arb_engine" > /dev/null; then
    echo "✅ MEVana is running"
else
    echo "❌ MEVana is not running"
    exit 1
fi

# Show recent logs
echo ""
echo "📋 Recent Activity (last 20 lines):"
echo "-----------------------------------"
tail -20 mevana.log 2>/dev/null || echo "No log file found"

# Check metrics endpoint
echo ""
echo "📈 Current Metrics:"
echo "------------------"
curl -s http://localhost:8088/metrics | grep -E "(arbitrage_opportunities|profit_total|execution_success)" | head -10

# Check wallet balances (if addresses file exists)
if [ -f "./trading-wallets/wallet-addresses.txt" ]; then
    echo ""
    echo "💰 Wallet Balances:"
    echo "------------------"
    while IFS= read -r line; do
        if [[ $line =~ :\ (.+)$ ]]; then
            address="${BASH_REMATCH[1]}"
            balance=$(solana balance "$address" 2>/dev/null || echo "Error")
            echo "$line - Balance: $balance"
        fi
    done < "./trading-wallets/wallet-addresses.txt"
fi
EOF
chmod +x monitor-trading.sh

print_status "Created monitoring script: monitor-trading.sh"

# 7. Create emergency stop script
cat > emergency-stop.sh << 'EOF'
#!/bin/bash
# Emergency stop for live trading

echo "🚨 EMERGENCY STOP TRIGGERED"
echo "============================"

# Kill the trading process
echo "Stopping trading process..."
pkill -f "hotpath_arb_engine"

# Send stop signal
echo "Sending emergency stop signal..."
curl -X POST http://localhost:8088/emergency/stop 2>/dev/null || true

echo "✅ Emergency stop completed"
echo "📋 Check logs and wallet balances immediately"
EOF
chmod +x emergency-stop.sh

print_status "Created emergency stop script: emergency-stop.sh"

# 8. Final validation and instructions
echo ""
echo "🎯 Setup Complete! Final Steps:"
echo "==============================="

print_warning "BEFORE ENABLING LIVE TRADING:"
echo ""
echo "1. 📝 Edit .env file to customize your settings"
echo "2. 🔑 Update JITO_AUTH_KEYPAIR_PATH in .env to: $JITO_KEYPAIR_PATH"  
echo "3. 💰 Fund your trading wallets (addresses in: $WALLETS_DIR/wallet-addresses.txt)"
echo "   - Hot wallets: 0.1 SOL each (for active trading)"
echo "   - Warm wallet: 1-5 SOL (for reserves)"
echo "   - Cold wallet: Keep most funds here (long-term storage)"
echo "4. 🧪 Test thoroughly with: ./start-dry-run.sh"
echo "5. 📊 Monitor with: ./monitor-trading.sh"
echo ""

print_status "Setup completed successfully!"
print_warning "Remember: Start with tiny position sizes and scale up gradually!"

echo ""
print_info "Quick Start:"
echo "  Test Mode:  ./start-dry-run.sh"
echo "  Live Mode:  ./start-live-trading.sh"  
echo "  Monitor:    ./monitor-trading.sh"
echo "  Emergency:  ./emergency-stop.sh"