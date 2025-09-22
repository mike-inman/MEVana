# Flash Loan Devnet Setup (Solend WSOL Host Fee)

Created: 2025-09-22

This short guide explains, in plain language, how to prepare wrapped SOL (`WSOL`) accounts and configuration so the flash-loan plugins have somewhere to send their fees on Solana devnet. Think of it like setting up special piggy banks that each loan provider can pay into after every borrow.

## What We Are Doing (Explained Simply)

- **Solana** has a practice network called **devnet** where we can test things without risking real money.
- **WSOL** is just SOL (the native coin) wrapped so it behaves like a token. Some programs, like Solend, expect fees to be paid in this wrapped form.
- Solend needs a token account (our “piggy bank”) where it can drop host fees. We create that account, put a little SOL inside it, and tell our app where to find it.

## Before You Start

- Solana CLI is installed and the default keypair lives at `~/.config/solana/id.json`.
- You are fine using devnet (the test network). If you are on another cluster, run:
  ```bash
  solana config set --url https://api.devnet.solana.com
  ```

## Step-by-Step (Solend WSOL Host Fee)

1. **Look up the WSOL reserve data (optional but good context)**
   ```bash
   curl -s "https://api.solend.fi/v1/config?deployment=devnet" \
     | jq '.markets[] | select(.name=="main") | .reserves[] | select(.asset=="SOL")'
   ```
   This shows the Solend addresses the plugin already uses in code. You do not need to copy them—just know we are working with the same reserve.

2. **Create a WSOL token account**
   ```bash
   spl-token create-account So11111111111111111111111111111111111111112
   ```
   - Keep the printed address; in our example it was
     `AJVS4MHtdmwnKAkKbuX7FUKVwhuEM1woVRVctS4Gi33S`.
   - This is the host-fee account Solend will pay into.

3. **Airdrop SOL to the account owner (your keypair)**
   ```bash
   solana airdrop 1 FvmBWiV16DGKt1uA4h86h3Y8G2DyQaMwGSv8uzmppMoD
   ```
   Replace the public key if your wallet is different. Devnet sometimes rate-limits; wait a minute and retry if needed.

4. **Wrap the SOL into WSOL using the new token account**
   ```bash
   spl-token sync-native --address AJVS4MHtdmwnKAkKbuX7FUKVwhuEM1woVRVctS4Gi33S \
     --url https://api.devnet.solana.com
   ```
   This converts any SOL sitting in the account owner’s wallet into WSOL inside the token account. After this step, the balance is visible with:
   ```bash
   spl-token balance --address AJVS4MHtdmwnKAkKbuX7FUKVwhuEM1woVRVctS4Gi33S \
     --url https://api.devnet.solana.com
   ```

5. **Tell the app where to send host fees**
   Add the account to your environment file (`.env` or `infrastructure/.env`):
   ```bash
   SOLEND_WSOL_HOST_FEE_ACCOUNT=AJVS4MHtdmwnKAkKbuX7FUKVwhuEM1woVRVctS4Gi33S
   ```
   The Solend plugin reads this value during startup. If it is missing, the fees go to Solend’s default receiver instead of your account.

## Quick Recap

1. Pick devnet (`solana config set ...`).
2. Create a WSOL token account (`spl-token create-account ...`).
3. Airdrop SOL to your wallet.
4. Wrap the SOL into that token account (`spl-token sync-native ...`).
5. Store the account address in `SOLEND_WSOL_HOST_FEE_ACCOUNT`.

Now the MEVana flash-loan plugin will direct all Solend host fees on devnet into your WSOL account. You can repeat the wrap step whenever you need to top it up.

## Marginfi Provider (Mainnet Addresses)

- Marginfi flash loans use fixed mainnet addresses baked into the code (see `marginfi/instructions.rs`). There is no optional setup on devnet right now, so you simply need to keep your trading wallet funded.
- If you want to dry-run quotes locally, the provider already supports SOL with a 0.05% fee and a 50 SOL cap.
- No extra environment variables are required.

## Port Finance Provider (Quotes + Receiver)

- The Port Finance plugin can quote WSOL loans on devnet using the built-in reserve values (`port_finance/mod.rs`).
- Full execution requires deploying a custom receiver program (Port’s program enforces this). Set `FLASH_LOAN_RECEIVER_PROGRAM_ID` to the deployed program id so the executor includes the hand-off instruction in the flash-loan plan.
- Until the receiver is implemented on-chain, the instruction is a no-op placeholder—use it for discovery/testing and update the program once the CPI flow is ready.

## Jet Protocol Provider (Env-Driven Setup)

Jet needs us to supply the reserve addresses through environment variables before it will register. Gather the latest devnet values from Jet’s docs or CLI and export them:

```bash
export JET_LENDING_MARKET=<pubkey>
export JET_RESERVE=<pubkey>
export JET_RESERVE_LIQUIDITY_SUPPLY=<pubkey>
export JET_FEE_RECEIVER=<pubkey>
# optional override if the PDA derivation is not what you expect
# export JET_MARKET_AUTHORITY=<pubkey>
export FLASH_LOAN_RECEIVER_PROGRAM_ID=<deployed_receiver_program>
```

Once these are set the Jet provider will register and return quotes. With `FLASH_LOAN_RECEIVER_PROGRAM_ID` defined, the executor also emits the receiver instruction so the on-chain program can perform the borrow/repay cycle when it is ready. Keep the values in your `.env` so the aggregator picks them up automatically.

## Testing the Providers

- Solend and Marginfi can both produce full borrow/repay plans in local tests (`cargo test -p flash-loan-providers`).
- Port Finance and Jet currently emit quotes only; their tests verify env/config validation so we know when they are ready for the next implementation step.
- When you are ready to ship full execution for those providers, follow `FLASH_LOAN_RECEIVER_PROGRAM.md` to build the shared CPI receiver program.

Remember: devnet is for practice. When you are ready to point at mainnet, replace the devnet WSOL account with a secure production wallet and remove any `solana airdrop` steps.
