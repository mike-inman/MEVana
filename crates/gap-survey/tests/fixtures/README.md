# 2026-10-31 CDT — Mainnet pool-state fixtures

These are unmodified public account data bytes from one Confirmed `getMultipleAccounts` read at context slot 463747556 through the approved Tatum endpoint. No credential, URL with a token, transaction, wallet or private account data is included.

| File | Account | Owner program | Raw bytes |
| --- | --- | --- | ---: |
| `orca_sol_usdc.bin` | `Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE` | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | 653 |
| `raydium_sol_usdc.bin` | `3ucNos4NbumPLZNWztqGHNFFgkHeRMBQAVemeeomsUxv` | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` | 1544 |
| `meteora_sol_usdc.bin` | `5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6` | `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` | 904 |
| `raydium_config.bin` | `3h2e43PunVA5K34vwKCLHWhZF4aZpyaC9RmxvshGAQpL` | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` | 117 |

The same read produced approximately 95.8049 USDC/SOL from Orca, 95.8063 from Raydium, and 95.7963 from Meteora using integer/decimal scaling. Tests verify the exact binary lengths, discriminators, mints and price agreement within 1%.
