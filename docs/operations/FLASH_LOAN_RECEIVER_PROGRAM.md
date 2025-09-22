# Flash Loan Receiver Program Outline

Created: 2025-09-22

Port Finance and Jet Protocol both require a lightweight “flash loan receiver” program. The lender insists that every flash-loan bundle invoke an on-chain contract owned by you; that program performs the borrow, executes your trading logic, and then sends the borrowed funds plus fee back to the reserve in one transaction. This note explains the required accounts and the project structure so the dev team can implement it quickly.

## Why We Need It

- **Solend and Marginfi** already expose CPI hooks that accept direct borrowers; our executor can build the borrow/repay instructions directly.
- **Port Finance and Jet** check that the borrower is a program they know how to call (a receiver). Until we deploy that program, any attempt to build a plan will end with “receiver program required”.

## High-Level Design

1. **Program Language**: Anchor is the fastest path (Jet already exposes Anchor-compatible interfaces), but a plain solana-program crate works too. Keep it minimal.
2. **Entrypoint Flow**
   - Instruction `execute_flash_loan` receives the flash-loan configuration, the trading payload (e.g. serialized arbitrage plan), and any extra accounts.
   - Inside, invoke the lender’s borrow instruction via CPI.
   - Run the trading logic (either inline or by CPIs into our existing executor components).
   - Invoke the lender’s repay instruction, making sure the fee account has the required lamports.
   - Emit events/logging for observability.
3. **Account Validation**
   - Verify the WSOL token accounts match the reserve metadata (the same accounts we place in `JET_RESERVE`, `JET_RESERVE_LIQUIDITY_SUPPLY`, etc.).
   - Check the authority signer matches the executor wallet (~`id.json` in devnet).
4. **Safety Switches**
   - Add a configuration account with flags like `enabled`, `max_amount`, `allowed_providers` so we can kill the program quickly if needed.
   - Include per-provider fee sanity checks to avoid overpaying.

## Suggested Project Layout

This repository already ships with a starter crate at `crates/flash-loan-receiver/` that contains:

- `build_execute_flash_loan_ix` – client helper used by the Port/Jet providers.
- `process_instruction` – a minimal on-chain entrypoint that simply logs the payload today. Extend this function with the real borrow/trade/repay logic.

You can keep the code in that crate or split it into a dedicated `programs/` directory later—just remember to update `FLASH_LOAN_RECEIVER_PROGRAM_ID` when you redeploy.

## Devnet Testing Checklist

1. Deploy the program ID and note it in `.env` (e.g. `FLASH_LOAN_RECEIVER_PROGRAM_ID`).
2. Update the Port/Jet providers to insert the receiver program when building the plan.
3. Spin up `solana-program-test` integration tests that:
   - Fund mock reserves with WSOL.
   - Execute `execute_flash_loan` with a dummy trading CPI (e.g. transfer lamports back and forth).
   - Assert the loan is repaid plus fee.
4. Once the tests pass, publish the ID to devnet and run an on-chain dry run using the executor service.

## Activation Steps (Devnet)

1. Build & deploy the receiver program (`anchor deploy` or `cargo build-bpf` + `solana program deploy`).
2. Export the new program ID and any configuration accounts into `.env`.
3. Modify the Port Finance and Jet provider implementations to call the receiver program instead of returning “not supported”.
4. Re-run `cargo test -p flash-loan-providers` and executor integration tests.
5. Perform a full devnet end-to-end run, confirm metrics in Grafana, then schedule the mainnet deployment plan.

### Building & Deploying the Starter Program

Use the provided crate as a starting point:

```bash
# Build for SBF (BPF) targets
cargo build-sbf --manifest-path crates/flash-loan-receiver/Cargo.toml \
  --sbf-out-dir target/deploy

# Deploy to devnet (replace KEYPAIR/path as needed)
solana program deploy target/deploy/flash_loan_receiver.so \
  --keypair ~/.config/solana/id.json \
  --url https://api.devnet.solana.com

# Update .env with the new program id
export FLASH_LOAN_RECEIVER_PROGRAM_ID=<program_id_printed_by_deploy>
```

The repository currently uses the placeholder program id `26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg` so tests can run. Replace it with the id printed by `solana program deploy` once your real receiver is uploaded.

With this outline in place we can implement the shared receiver program next. Once that lands, both Port Finance and Jet will graduate from “quote-only” to “full flash-loan execution”.
