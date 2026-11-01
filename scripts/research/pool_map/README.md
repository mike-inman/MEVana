# 2026-10-31 — Pool map sample tool

This extends the existing research scripts using standard-library Python Decimal. Build the existing public-account helper with rustup Cargo before collection:

```sh
PATH="$HOME/.cargo/bin:$PATH" cargo build -p gap-survey --bin pool_snapshot --offline
python3 -m unittest discover -s scripts/research/pool_map -v
python3 scripts/research/pool_map/sample.py collect UNIQUE_RUN_ID
python3 scripts/research/pool_map/sample.py replay ~/MEVanaData/pool-map/UNIQUE_RUN_ID
scripts/research/load-pool-map.sh ~/MEVanaData/pool-map/UNIQUE_RUN_ID
```

Collection is a single foreground invocation, capped below 20 minutes, 300 public calls and 20 Tatum account reads. Existing run directories are refused. Raw public response bodies and statuses are retained outside Git. Saved bundles permit offline replay without repeating collection. Money values use Decimal and serialize as exact strings. Request pacing and retry handling use only public APIs. The account helper uses the existing named-value credential loader without sourcing or printing credentials.

The shortlist is capped at 40 pairs and three pools each. The first failed R1–R5 requirement is retained. USDC and USDT have named issuer-freeze exemptions; other restricted mint extensions fail. R5 unknowns stay for review. Candidate order is lowest turnover among pairs above the median of deepest eligible pool liquidity. A seeded comparison group excludes selected candidates. OnChain active-liquidity depth estimates ignore tick crossings and use Reported token conversions. They are estimates, not execution quotes. Public lists and source errors make coverage partial.
