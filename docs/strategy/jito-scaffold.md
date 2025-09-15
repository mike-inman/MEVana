# Jito Scaffold (Canary)
Last updated: 2025-09-15

## Components
- Bundle builder: assemble txn with accounts, compute budget, instructions
- Simulator: dry-run against RPC (or mock), ensure positive expected net PnL
- Tip policy: interface returning tip lamports based on acceptance model

## Canary Mode
- JITO_CANARY=1 → log bundle + tip only; do not submit
- Collect acceptance stats over time to calibrate tip curve

## Production
- Private connection to Jito relays
- Submit only if expected net PnL > 0 and acceptance probability above threshold
