# MEVana Live Trading Setup Guide
Last updated: 2025-09-15

## Overview

MEVana includes a complete live trading infrastructure that can be activated with a single environment variable change. This guide walks you through preparing everything needed for live trading activation.

## 🚨 **IMPORTANT SAFETY WARNING** 🚨

**Live trading involves real money and real risk. Always:**
- Start with tiny position sizes (0.001-0.01 SOL)
- Test thoroughly in dry-run mode first
- Monitor the system continuously when live
- Keep most funds in cold storage
- Have emergency stop procedures ready
- Never risk more than you can afford to lose

## Quick Start

The entire live trading system is designed for **one-click activation**. Here's the complete process:

### 1. Initial Setup

```bash
# Copy the live trading configuration template
cp .env.live-trading-template .env

# Run the automated setup script
./setup-live-trading.sh
```

This script will:
- Generate all necessary wallet keypairs
- Create Jito authentication keypair
- Set up monitoring and emergency scripts
- Validate your environment

### 2. Configuration

Edit your `.env` file to customize settings:

```bash
# Core safety settings
ENABLE_LIVE_EXECUTION=false        # Keep false until ready!
DRY_RUN=true                      # Safe testing mode
SIMULATION_MODE=true              # Paper trading only

# Position sizing (start conservative!)
MIN_POSITION_SIZE_SOL=0.001       # $0.20 minimum
MAX_POSITION_SIZE_SOL=0.01        # $2.00 maximum  
MAX_DAILY_LOSS_SOL=1.0           # $200 daily limit

# Wallet security limits
HOT_WALLET_MAX_BALANCE=0.1        # 0.1 SOL per hot wallet
WARM_WALLET_MAX_BALANCE=5.0       # 5 SOL reserve wallet
COLD_WALLET_MAX_BALANCE=100.0     # 100 SOL cold storage

# Jito MEV protection (REQUIRED for live trading)
JITO_ENABLED=true
JITO_AUTH_KEYPAIR_PATH=./jito-auth-keypair.json
JITO_TIP_LAMPORTS=10000           # 0.00001 SOL tip per bundle
```

### 3. Fund Your Wallets

After setup, fund the generated wallets with small amounts:

```bash
# Check wallet addresses
cat ./trading-wallets/wallet-addresses.txt

# Fund wallets (use Solana CLI or send from your main wallet)
# Hot wallets: 0.1 SOL each (for active trading)  
# Warm wallet: 1-5 SOL (for reserves)
# Cold wallet: Keep most funds here (long-term storage)
```

### 4. Test in Dry-Run Mode

**ALWAYS test first before going live:**

```bash
# Start in safe testing mode
./start-dry-run.sh

# Monitor in another terminal
./monitor-trading.sh

# Check logs and metrics at http://localhost:8088/metrics
```

### 5. **One-Click Live Trading Activation** 🚀

When you're ready for live trading:

```bash
# Edit .env file
ENABLE_LIVE_EXECUTION=true    # THE ONLY CHANGE NEEDED!
DRY_RUN=false
SIMULATION_MODE=false

# Start live trading
./start-live-trading.sh
```

## System Architecture

### Multi-Tier Wallet Security

The system uses a three-tier wallet architecture for maximum security:

- **Hot Wallets** (2x): Active trading wallets with strict balance limits
- **Warm Wallet** (1x): Reserve funds for rebalancing hot wallets
- **Cold Wallet** (1x): Long-term storage, unlimited balance

### Jito MEV Protection

All live trades are submitted as Jito bundles for MEV protection:
- Prevents front-running and sandwich attacks
- Configurable tip amounts for priority
- Success rate monitoring and optimization
- Automatic retry logic with backoff

### Smart Execution System

- **AI Confidence Scoring**: Only executes high-confidence opportunities
- **Competition Detection**: Adjusts position sizes based on competition
- **Risk Management**: Circuit breakers, daily loss limits, slippage protection
- **Multi-Venue Support**: Raydium, Orca, Meteora, Phoenix with per-venue limits

## Safety Features

### Circuit Breakers
- Halt trading after consecutive failures
- Daily loss limits with automatic shutdown
- Rate limiting (max trades per minute)
- Emergency stop mechanisms

### Monitoring & Alerts
- Real-time metrics at `/metrics` endpoint
- Comprehensive logging of all activities
- Optional Slack/Discord webhook alerts
- Database logging for audit trails

### Emergency Controls
```bash
# Emergency stop (immediately halt all trading)
./emergency-stop.sh

# Monitor system status
./monitor-trading.sh

# Check wallet balances and recent activity
solana balance $(cat ./trading-wallets/wallet-addresses.txt)
```

## Configuration Reference

### Core Settings

| Setting | Description | Recommended Value |
|---------|-------------|-------------------|
| `ENABLE_LIVE_EXECUTION` | Master switch for live trading | `false` (until ready) |
| `MIN_POSITION_SIZE_SOL` | Minimum trade size | `0.001` (start small) |
| `MAX_POSITION_SIZE_SOL` | Maximum trade size | `0.01` (scale up gradually) |
| `MAX_DAILY_LOSS_SOL` | Daily loss limit | `1.0` (adjust to risk tolerance) |
| `MIN_CONFIDENCE_FOR_EXECUTION` | Minimum AI confidence | `0.8` (high confidence only) |

### Risk Management

| Setting | Description | Recommended Value |
|---------|-------------|-------------------|
| `MAX_CONSECUTIVE_FAILURES` | Circuit breaker threshold | `3` |
| `COOLING_OFF_PERIOD_SECONDS` | Wait time after failure | `30` |
| `MAX_TRADES_PER_MINUTE` | Rate limiting | `10` |
| `MAX_SLIPPAGE_BPS` | Maximum slippage | `50` (0.5%) |

### Venue Limits

Individual position limits per DEX for safety:

```env
MAX_POSITION_RAYDIUM=0.005     # $1 max per Raydium trade
MAX_POSITION_ORCA=0.005        # $1 max per Orca trade  
MAX_POSITION_METEORA=0.002     # $0.40 max per Meteora trade
MAX_POSITION_PHOENIX=0.001     # $0.20 max per Phoenix trade
```

## Monitoring & Metrics

### HTTP Endpoints

- `http://localhost:8088/metrics` - Prometheus metrics
- `http://localhost:8088/performance` - Performance analytics
- `http://localhost:8088/ai/config` - AI system status
- `http://localhost:8088/wallet/security` - Wallet security dashboard

### Key Metrics to Monitor

1. **Arbitrage Opportunities**: Detected per minute
2. **Execution Success Rate**: Percentage of successful trades
3. **Net Profit**: After fees and gas costs
4. **Wallet Balances**: Ensure proper tier distribution
5. **AI Confidence Scores**: Quality of opportunities
6. **Jito Bundle Success**: MEV protection effectiveness

## Troubleshooting

### Common Issues

1. **"Jito auth keypair not found"**
   - Ensure `JITO_AUTH_KEYPAIR_PATH` points to generated keypair
   - Run `./setup-live-trading.sh` again if needed

2. **"Insufficient balance for trading"**  
   - Check wallet balances with `./monitor-trading.sh`
   - Fund hot wallets with at least minimum position size

3. **"Circuit breaker triggered"**
   - Check logs for execution failures
   - Reduce position sizes if market conditions are volatile
   - Verify RPC endpoint is responding properly

4. **"No arbitrage opportunities found"**
   - Check if streaming services are running
   - Verify DEX data feeds are connected
   - Monitor `/metrics` endpoint for data flow

### Emergency Procedures

If something goes wrong:

```bash
# 1. IMMEDIATE STOP
./emergency-stop.sh

# 2. Check system status
./monitor-trading.sh

# 3. Verify wallet balances
solana balance $(head -1 ./trading-wallets/wallet-addresses.txt | cut -d: -f2 | xargs)

# 4. Check recent logs
tail -50 mevana.log

# 5. Review recent transactions on Solscan/Explorer
```

## Development & Testing

### Testing New Strategies

Always test changes in dry-run mode:

```bash
# 1. Make code changes
# 2. Rebuild
cargo build --release -p hotpath_arb_engine

# 3. Test in simulation
ENABLE_LIVE_EXECUTION=false DRY_RUN=true ./target/release/hotpath_arb_engine

# 4. Monitor results
./monitor-trading.sh
```

### Configuration Validation

The system validates all settings on startup:

```bash
# Check configuration without starting
RUST_LOG=info ./target/release/hotpath_arb_engine --validate-config
```

## Security Best Practices

1. **Never commit private keys** to version control
2. **Use separate keypairs** for each environment (dev/staging/prod)
3. **Regularly rotate** Jito auth keypairs
4. **Monitor wallet activity** continuously
5. **Keep cold wallet offline** when not rebalancing
6. **Use strong passwords** for any encrypted keypairs
7. **Backup all keypairs** securely and separately
8. **Test recovery procedures** regularly

## Scaling Up

Once comfortable with the system:

1. **Gradually increase** position sizes
2. **Add more hot wallets** for higher throughput  
3. **Tune AI parameters** based on performance data
4. **Optimize venue-specific** settings
5. **Consider flash loan integration** for larger opportunities
6. **Add custom strategies** through the plugin system

Remember: **Start small, scale gradually, monitor continuously**. The system is designed to be safe by default - take advantage of all the built-in protections!
