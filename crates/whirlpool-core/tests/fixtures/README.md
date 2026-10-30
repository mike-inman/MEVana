# 2026-10-29 22:26 CDT — Public Whirlpool decoder fixtures

These are zstd-compressed, unmodified public account data bytes from bounded Sodae dev/test Geyser captures. They test account decoding only; they are not evidence of an opportunity or profitability. No credential or transaction payload is included.

| File | Account pubkey | Slot | Raw bytes | Capture session |
| --- | --- | ---: | ---: | --- |
| `usdc_usdt_pool.bin.zst` | `4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4` | 463130589 | 653 | `capture-1793330164981129000` |
| `usdc_usdt_tick_array.bin.zst` | `FqFkv2xNNCUyx1RYV61pGZ9AMzGfgcD8uXC9zCF5JKnR` | 463131460 | 9,988 | `capture-1793330490539629000` |

The TickArray stores the pool pubkey at byte offset 9,956. Both source captures remain outside the repository under `~/MEVanaData/captures/handoff09-20261029/`.
