# MEVana Funding Setup Guide
Last updated: 2025-09-15

## 💰 **How to Add Funds to MEVana for Trading**

### 🏦 **Current Wallet Infrastructure**

✅ **What's Built**:
- WalletService with balance tracking and risk management
- Multi-wallet support (primary + backup wallets)
- Daily limits and risk controls
- Token balance management across all SPL tokens
- Integration with executor service

⚠️ **What Was Just Fixed**:
- Real Solana keypair loading (was placeholder)
- Proper public key extraction from wallet files
- Added wallet setup verification tool

---

## 🚀 **Quick Start - Fund Your Trading Wallet**

### **Step 1: Create/Setup Wallet**

```bash
# Option A: Create new dedicated trading wallet (RECOMMENDED)
solana-keygen new --outfile ~/.config/solana/mevana-trading.json

# Option B: Use existing wallet (COPY to avoid conflicts)
cp /path/to/your/existing-wallet.json ~/.config/solana/mevana-trading.json

# Get your new trading wallet address
TRADING_ADDRESS=$(solana-keygen pubkey ~/.config/solana/mevana-trading.json)
echo "🏦 Your MEVana Trading Address: $TRADING_ADDRESS"
```

### **Step 2: Configure Environment**

```bash
# Set wallet path for MEVana
export WALLET_KEYPAIR_PATH="~/.config/solana/mevana-trading.json"

# Optional: Set backup wallets  
export WALLET_BACKUP_PATHS="[]"

# Set balance limits (adjust as needed)
export MAX_WALLET_BALANCE_SOL="100.0"  # Maximum SOL to keep in trading wallet
export MIN_WALLET_BALANCE_SOL="10.0"   # Minimum SOL before stopping trades
```

### **Step 3: Fund Your Wallet**

```bash
# Your trading wallet address
echo "Send SOL to: $TRADING_ADDRESS"

# Recommended funding amounts:
# 🧪 TEST MODE: 5-10 SOL (safe for testing)
# 🔥 LIVE TRADING: 50-100 SOL (minimum for profitable arbitrage)
# 💎 FULL TRADING: 500+ SOL (for maximum opportunities)
```

### **Step 4: Verify Setup**

```bash
# Build and run the wallet verification tool
cargo build -p executor-service --bin wallet-setup
cargo run -p executor-service --bin wallet-setup

# Check balance manually
solana balance ~/.config/solana/mevana-trading.json
```

---

## 🔧 **Detailed Configuration**

### **Environment Variables**

```bash
# Required - Wallet Configuration
export WALLET_KEYPAIR_PATH="~/.config/solana/mevana-trading.json"
export WALLET_BACKUP_PATHS="[]"  # JSON array of backup wallet paths

# Risk Management
export MAX_WALLET_BALANCE_SOL="100.0"      # Max SOL in wallet
export MIN_WALLET_BALANCE_SOL="10.0"       # Min SOL before stopping
export DAILY_LOSS_LIMIT_SOL="5.0"          # Max daily losses
export MAX_POSITION_SIZE_SOL="20.0"        # Max per-trade size
export MAX_CONCURRENT_TRADES="5"           # Max simultaneous trades

# Trading Parameters  
export MIN_PROFIT_THRESHOLD_BPS="25"       # Min 0.25% profit required
export MAX_SLIPPAGE_BPS="100"              # Max 1% slippage allowed
export MAX_EXECUTION_TIME_MS="10000"       # 10 second timeout

# Flash Loan Limits
export MAX_FLASH_LOAN_AMOUNT_SOL="50.0"    # Max flash loan size
export FLASH_LOAN_TIMEOUT_MS="5000"        # 5 second timeout
```

### **Config File Location**
MEVana loads configuration from environment variables. You can create a `.env` file:

```bash
# Create config file
cat > ~/.config/solana/mevana.env << EOF
WALLET_KEYPAIR_PATH=~/.config/solana/mevana-trading.json
MAX_WALLET_BALANCE_SOL=100.0
MIN_WALLET_BALANCE_SOL=10.0
DAILY_LOSS_LIMIT_SOL=5.0
MAX_POSITION_SIZE_SOL=20.0
MIN_PROFIT_THRESHOLD_BPS=25
EOF

# Source before running MEVana
source ~/.config/solana/mevana.env
```

---

## 💡 **Funding Strategies by Mode**

### **🧪 Test Mode** (Start Here!)
- **Amount**: 5-10 SOL
- **Purpose**: Validate system without risk
- **Duration**: Run for 4+ hours before live trading
- **Risk**: Zero (simulated trades only)

```bash
export EXECUTOR_TEST_MODE=true
export MAX_POSITION_SIZE_SOL="2.0"  # Small positions
```

### **🔥 Live Trading** (After Test Success)
- **Amount**: 50-100 SOL minimum  
- **Purpose**: Real profitable arbitrage
- **Strategy**: Start small, scale gradually
- **Risk**: Managed by built-in circuit breakers

```bash
export EXECUTOR_TEST_MODE=false
export MAX_POSITION_SIZE_SOL="20.0"  # Larger positions
```

### **💎 Full Production** (After Live Validation)
- **Amount**: 500+ SOL
- **Purpose**: Maximum MEV capture
- **Strategy**: Full market coverage across all venues
- **Risk**: Professional-grade risk management

---

## 🔐 **Security Best Practices**

### **Wallet Security**
- ✅ Use dedicated wallet for trading only
- ✅ Keep backup of keypair in secure location
- ✅ Never share private key or keypair file
- ✅ Use hardware wallet for main funds storage
- ✅ Test with small amounts first

### **Operational Security**
- ✅ Monitor balances and PnL regularly
- ✅ Set appropriate daily loss limits
- ✅ Use test mode extensively before live trading
- ✅ Keep main wallet separate from trading wallet
- ✅ Regular backup of configuration and logs

### **Risk Management**
```bash
# Conservative Settings (Recommended Start)
export DAILY_LOSS_LIMIT_SOL="2.0"       # Max $400 daily loss at $200/SOL
export MAX_POSITION_SIZE_SOL="5.0"      # Max $1000 per trade
export MIN_PROFIT_THRESHOLD_BPS="50"    # Min 0.5% profit required

# Aggressive Settings (After Proven Success)
export DAILY_LOSS_LIMIT_SOL="10.0"      # Max $2000 daily loss
export MAX_POSITION_SIZE_SOL="25.0"     # Max $5000 per trade
export MIN_PROFIT_THRESHOLD_BPS="25"    # Min 0.25% profit required
```

---

## ⚡ **Quick Commands Reference**

```bash
# Check wallet balance
solana balance ~/.config/solana/mevana-trading.json

# View transaction history
solana transaction-history $(solana-keygen pubkey ~/.config/solana/mevana-trading.json)

# Monitor wallet in Explorer
ADDR=$(solana-keygen pubkey ~/.config/solana/mevana-trading.json)
echo "https://explorer.solana.com/address/$ADDR"

# Emergency: Send all funds back to main wallet
solana transfer --from ~/.config/solana/mevana-trading.json \
    YOUR_MAIN_WALLET_ADDRESS ALL --fee-payer ~/.config/solana/mevana-trading.json
```

---

## 🚨 **Emergency Procedures**

### **Stop All Trading Immediately**
```bash
# Kill all MEVana processes
pkill arb-engine
pkill executor-service
pkill whirlpool-live-stream

# Enable circuit breaker via HTTP
curl -X POST http://localhost:8081/emergency/circuit-breaker/enable
```

### **Withdraw All Funds**
```bash
# Check current balance
solana balance ~/.config/solana/mevana-trading.json

# Send to your safe wallet (replace YOUR_SAFE_WALLET)
solana transfer --from ~/.config/solana/mevana-trading.json \
    YOUR_SAFE_WALLET_ADDRESS ALL
```

---

## 📊 **Monitoring Your Funds**

### **Built-in Monitoring**
- **Grafana Dashboard**: http://localhost:3000
- **Metrics Endpoint**: http://localhost:8081/metrics  
- **Health Check**: http://localhost:8081/health

### **Key Metrics to Watch**
- Current SOL balance
- Daily PnL (profit/loss)
- Number of active trades
- Risk circuit breaker status
- Success rate of trades

### **Alerts to Set Up**
- Balance below minimum threshold
- Daily loss limit approached
- Consecutive trade failures
- Wallet health issues

---

## 🎯 **Next Steps After Funding**

1. **✅ Fund wallet with recommended amount**
2. **✅ Verify setup with wallet-setup tool**
3. **✅ Start in TEST MODE for validation**
4. **✅ Monitor performance for 4+ hours**
5. **✅ Switch to LIVE MODE with small positions**
6. **✅ Scale up gradually after success**

## 📞 **Need Help?**

- Check logs: `journalctl -f -u mevana`
- Monitor Grafana: http://localhost:3000
- View balance: `solana balance ~/.config/solana/mevana-trading.json`
- Emergency stop: `pkill arb-engine && pkill executor-service`

**Remember**: Always start with test mode and small amounts!
