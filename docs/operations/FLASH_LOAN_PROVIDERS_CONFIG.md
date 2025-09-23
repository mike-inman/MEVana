# Flash Loan Providers Configuration

Created: 2025-09-22  
Last updated: 2025-09-22

The flash-loan adapters (Save/Solend, Port Finance, Kamino, Marginfi) now read their reserve metadata and fee overrides from a JSON file referenced by the `FLASH_LOAN_PROVIDERS_CONFIG` environment variable. This page explains the format, recommended values, and where to place the file in each environment.

## Why a JSON config?

- Different environments (local dev, staging, mainnet) often use different reserves, liquidity caps, or host-fee accounts. Baking those values into the binary made updates painful.
- The JSON file lets us cap borrow size, specify reserve accounts, and adjust protocol-specific fees without recompiling.
- The executor *and* hotpath engine consume the same file, keeping quotes and plan assembly consistent.

## File Locations

| Environment | Suggested path | Notes |
|-------------|----------------|-------|
| Local dev   | `config/flash_loan_providers.local.json` (defaults to the committed devnet values in `config/flash_loan_providers.devnet.json`) | Export `FLASH_LOAN_PROVIDERS_CONFIG=$PWD/config/flash_loan_providers.local.json` before running Docker or `cargo run`. |
| CI          | `config/flash_loan_providers.ci.json` | Stored in repo (or pipeline secrets). The CI workflow should set the env var before launching tests. |
| Production  | `/opt/MEVana/config/flash_loan_providers.mainnet.json` (start from `config/flash_loan_providers.mainnet.template.json`) | Deployed alongside the compose stack; export the env var in the systemd unit or shell profile. |

## JSON structure

See `config/flash_loan_providers.sample.json` for a full example. At a high level:

```jsonc
{
  "save": {
    "program_id": "So1end…",
    "reserves": {
      "<token mint>": {
        "available_amount": 50000000000,
        "fee_bps": 30,
        "fee_lamports": 150000,
        "reserve": "<reserve account>",
        "liquidity_vault": "<reserve liquidity vault>",
        "fee_receiver": "<protocol fee vault>",
        "host_fee_receiver": "<optional host fee account>"
      }
    }
  },
  "port": { "program_id": "Port7…", "reserves": { … } },
  "kamino": { "program_id": "…", "reserves": { … } },
  "marginfi": { "program_id": "MFv2…", "reserves": { … } }
}
```

### Fields

- `program_id` *(optional)* – overrides the on-chain program id for this provider. If omitted we fall back to the baked-in default.
- `reserves` – keys are token mint addresses (base58). Each entry supports:
  - `available_amount` – when set, caps how much we ever borrow from that reserve (lamports).
  - `fee_bps` / `fee_lamports` – overrides the fee quote returned to the optimizer. Supply either or both.
  - `reserve`, `liquidity_vault`, `fee_receiver`, `host_fee_receiver` – account addresses required by Port/Kamino (Drive plan assembly). If a protocol does not need one of these fields you can omit it.

## Workflow

1. For devnet/local testing you can start from `config/flash_loan_providers.devnet.json` (already committed). Copy it to `config/flash_loan_providers.local.json` if you want to tweak values without modifying the shared file.
2. For production/staging clone `config/flash_loan_providers.mainnet.template.json`, fill in the real reserve accounts, and deploy it alongside the stack.
3. Export `FLASH_LOAN_PROVIDERS_CONFIG` before starting Docker or any binary that uses flash loans.
4. Commit environment-specific versions (`*.dev.json`, `*.ci.json`) as needed, keeping secrets out of the repo.

## Validation tips

- `cargo test -p flash-loan-providers` exercises plan building for Solend, Port, Kamino, and Marginfi using whatever overrides you set in the global config. If a required field is missing the tests will fail loudly.
- The executor logs the chosen provider, amount, and fee each time it builds a plan. Enabling debug logs (`RUST_LOG=debug`) helps confirm the config was loaded.
- Prometheus metrics `mevana_flash_loan_requests_total`, `mevana_flash_loan_successes_total`, and `mevana_flash_loan_failures_total` now increment whenever the aggregator interacts with providers, giving you quick feedback that the config is being consumed.

Keep this file under version control (or infrastructure management) so configuration drift between environments is minimized.
