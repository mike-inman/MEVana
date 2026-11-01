# 2026-10-31 23:51 CDT — Handoff 19 pool map sample closeout

Created: 2026-10-31 23:51 CDT

Base: local main c7c5d34a, containing c6aa0175. One sample `sample-20261031-19` finished in 730.1457171519869 seconds: 67 public API requests, 2 Tatum getMultipleAccounts calls, zero Geyser bytes. No collection limit was reached, and collection was not repeated. Raw responses, bundle, replay and screenshot stay under `~/MEVanaData/pool-map/sample-20261031-19/` outside Git.

GeckoTerminal and DexScreener both worked. Fourteen HTTP 429 responses were paced and retried at most twice; two GeckoTerminal endpoints remained partial. DexScreener has no documented top-by-exchange endpoint, so its search and recent-profile component is explicitly a sample. Source documentation and fixed choices are in the dated plan. No paid plan or account was used. Three sample rounds began at UTC epochs 1793507970, 1793508270, 1793508570 (300 seconds apart).

## Coverage and funnel

Collected 522 distinct pools and 354 pairs, including the 13 controls. Source exchange identifiers and pool counts: meteora: 150; meteora-damm-v2: 20; orca: 93; pancakeswap: 5; pancakeswap-v3-solana: 1; pump-fun: 37; pumpfun: 11; pumpswap: 58; raydium: 117; raydium-clmm: 10; raydium-launchlab: 20. Orca, Raydium, Meteora and PumpSwap are all covered. The sample is incomplete, particularly for less active pairs.

| Step | Entering | Rejected | Leaving / retained |
| --- | ---: | ---: | ---: |
| R1 | 354 | 301 | 53 |
| R2 | 53 | 29 | 24 |
| R3 | 24 | 2 | 22 |
| R4 | 22 | 4 | 18 |
| R5 | 18 | 0 | 18 |

Rejection reasons: {'fewer_than_two_pools': 301, 'liquidity_below_floor': 25, 'freeze_authority': 1, 'NewPool': 2, 'liquidity_unknown': 4, 'token2022_restricted_extension': 3}.
R5 unknown fee comparisons retained for review: 17. These are review candidates, not measured passes of the fee condition. The provisional liquidity threshold remains 20000 and age threshold seven days. Reported liquidity is not depth.

USDC and USDT are the named issuer-freeze exemptions. Their freeze flags remain stored. Other active freeze authorities and the specified Token-2022 extensions are rejected. Mint parsing uses OnChain account reads; source prices, volume and liquidity remain Reported. Near-price depth estimates use OnChain active liquidity with Reported USD conversion, ignore tick crossings, and do not imply fillable depth.

## Candidates (9)

Candidate rule: lowest turnover among pairs at or above median deepest-pool reported liquidity; no composite score. Comparison uses SHA-256 of run id and excludes candidates.
| Mint pair | First failure / review | Turnover | Gap bps | Movement bps | Liquidity imbalance | Combined fee bps | Depth USD estimate |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump|So11111111111111111111111111111111111111112 | Not measured | 0.0379 | 47.3560 | 39.4789 | 7.5712 | Not measured | 12502.8814 |
| FeR8VBqNRSUD5NtXAj2n3j1dAHkZHfyDktKuLXD4pump|So11111111111111111111111111111111111111112 | Not measured | 0.0458 | 20.1998 | 4.4924 | 74.2729 | Not measured | Not measured |
| J3NKxxXZcnNiMjKw9hYb2K4LUxgwB6t1FtPtQVsv3KFr|So11111111111111111111111111111111111111112 | Not measured | 0.1310 | 32.5556 | 37.9713 | 17.7060 | Not measured | 5139.3821 |
| 61V8vBaqAGMpgDQi4JcAwo1dmBGHsyhzodcPqnEVpump|So11111111111111111111111111111111111111112 | Not measured | 0.1469 | 34.6380 | 18.4886 | 146.6138 | Not measured | 108.8080 |
| Dz9mQ9NzkBcCsuGPFJ3r1bS4wgqKMHBPiVuniW8Mbonk|So11111111111111111111111111111111111111112 | Not measured | 0.1753 | 80.7339 | 51.3385 | 5.7615 | Not measured | 6796.0295 |
| 6GmAFSYs4gk3FDao5FzzySQpPZaWsa4rUJHacpMpUNgx|So11111111111111111111111111111111111111112 | Not measured | 0.2178 | 119.7605 | 14.8920 | 1.4692 | Not measured | 17331.7063 |
| CARDSccUMFKoPRZxt5vt3ksUbxEFEcnZ3H2pd3dKxYjp|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | Not measured | 0.5082 | 32.5336 | 12.1877 | 43.5437 | Not measured | 484.3849 |
| 98sMhvDwXj1RQi5c5Mndm3vPe9cBqPrbLaufMXFNMh5g|So11111111111111111111111111111111111111112 | Not measured | 1.4932 | 28.1599 | 24.9066 | 7.3930 | Not measured | 723.6287 |
| 3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh|So11111111111111111111111111111111111111112 | Not measured | 2.4507 | 8.9795 | 18.1126 | 17.0277 | Not measured | 835.8499 |
## Comparison (7)


| Mint pair | First failure / review | Turnover | Gap bps | Movement bps | Liquidity imbalance | Combined fee bps | Depth USD estimate |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh|7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs | Measured R5 pass | 0.0057 | 47.0562 | 6.9379 | 6.7840 | 46.0000 | 109.3222 |
| 3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | Not measured | 0.7382 | 12.7957 | 9.9932 | 1.0462 | Not measured | 1454.7497 |
| 6GmAFSYs4gk3FDao5FzzySQpPZaWsa4rUJHacpMpUNgx|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | Not measured | 3.9927 | 7.8156 | 7.8156 | 2.1091 | Not measured | 8187.6956 |
| 9cRCn9rGT8V2imeM2BaKs13yhMEais3ruM3rPvTGpump|So11111111111111111111111111111111111111112 | Not measured | 0.4581 | 20.9544 | 55.3382 | 3.6061 | Not measured | Not measured |
| C5xph2yL6gMdxmuLGxJBkGpmuDFfyi5fqkBNhvL6pump|So11111111111111111111111111111111111111112 | Not measured | 5.8140 | 89.1771 | 154.5344 | 3.0224 | Not measured | Not measured |
| Dz9mQ9NzkBcCsuGPFJ3r1bS4wgqKMHBPiVuniW8Mbonk|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | Not measured | 0.0014 | 122.6054 | 38.4763 | 48.5447 | Not measured | Not measured |
| So11111111111111111111111111111111111111112|oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp | Not measured | 1.3572 | 111.6458 | 40.2800 | 1.1003 | Not measured | 2541.3246 |
## Control (13)


| Mint pair | First failure / review | Turnover | Gap bps | Movement bps | Liquidity imbalance | Combined fee bps | Depth USD estimate |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263|So11111111111111111111111111111111111111112 | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm|So11111111111111111111111111111111111111112 | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3 | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|So11111111111111111111111111111111111111112 | Not measured | 0.8116 | 31.7308 | 51.3133 | 1.2775 | Not measured | 173472.5254 |
| EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB|So11111111111111111111111111111111111111112 | Not measured | 0.5098 | 38.2905 | 58.0601 | 1.6317 | Not measured | 1274.8223 |
| HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3|So11111111111111111111111111111111111111112 | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN|So11111111111111111111111111111111111111112 | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| So11111111111111111111111111111111111111112|jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL | R1 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |

## NewPool list

152 recent pools are separate from candidates:

| Pool address | Source venue |
| --- | --- |
| 1xiwcqXMkPXgo1vBjK3Cx4kt4WoYLQsCs61aAf3Xr6R | pumpswap |
| 25rG1gAiVaw4Cv6D7naA7jYhkrE2vgFyktfAHVV9TKfg | pump-fun |
| 2JSwDsyXyB3w9x9CWkmbn8fp76WzJ8BKEjB6tHVBifKS | pumpfun |
| 2NwbsLE3q2pgPWhY29bDefF8qKXZCApbWcsELsj1SXKN | pump-fun |
| 2ZvxeVP9UXbCS1P3wjgUGz4xKHs9T7Ng75v2BoKMN2zM | meteora |
| 2ip1LYJ56dYxWS7gfLdNPww5Qs4TQsAR4FJ23EfKVcNr | pump-fun |
| 2rmaFtFZmJtLXdPgRvk417RLc9GBD3jGbAqtttmfG3AF | pumpswap |
| 2yyP1cEXm4VZEHSGpqnWGxcZ3kSSVAK7m3j5nsKmFAhE | pumpswap |
| 34BLb55VRxax3gJx7SGMxHctsjjkUYXvnamKxsMUiWwT | meteora-damm-v2 |
| 36CnJXNugbqqGT8HELnuEhpz1Grg84m4DB1Qv1JfYZY9 | raydium-launchlab |
| 39SJ3DQezdKpUL8stn9qzwR5UwVhsuqcxxTUGmnDvjfe | pumpswap |
| 3CxfyALUoK8SZf35g2PKd3z2kbr6dxk7rdB9Pfn5F5Z3 | pumpswap |
| 3EaMfsnNogSDqvqPgk9ApFdqkxb74bfJS4AHgLvSxY3z | pump-fun |
| 3HrvZKoagxo2pF52CPTzk4g3iLgmr6Xm3bxbBu9xTttf | meteora-damm-v2 |
| 3SD4zwQnpCLfmeNqBsCQvsPxBatKWkUPBV7hM7dy7Y3z | meteora |
| 3dtgUbKrF7mxgKTruAMgbYcJH5reTKhrtoBn2ybUVaGf | pumpswap |
| 3h8d6M9rpiu65wQ5vfHcpNPsPipfsMaqFtxCKJwg2xby | raydium-launchlab |
| 3iv77wPFFfF9VW1ZnPpTC6g8AiJyKwv56UcVY5UpRSxt | pump-fun |
| 3mF3TBvNUQWUBPu4mMshDgFbFGauHWjactYspeSbpxRH | pump-fun |
| 3nkrAXJzyxb3gTpYK9uWZMq4DQ9nmWobrvwyLFC1rkvm | pumpswap |
| 3oeWnoFby56KAHR4M9fbPHXcY7MGYzRM16KF8dqgwmit | pump-fun |
| 3qyrT6YD7Qx2t5eGxqrjcJGKCbBRLM8NTACHRez1t4bA | raydium-launchlab |
| 3sQ1P2YNZVreGsW4CkMMKThEc5oFxxsdhbR1kczDZ6HB | meteora |
| 478wURDrDWeckP5zJaDoXjLVEMSKL6MJJ8D59wGae3TX | meteora |
| 4c5yD2CUHMbJyxQ4e8Sod6Z6WHFekYZDdN12EK811tMB | meteora-damm-v2 |
| 4oyDHv8RFE8bLP6t4nzqLNPfmacD8wo9P4ZNSzyV4vQj | raydium |
| 4s34MWRCNPKRoBAeqhf2PK9LzmrFdAE5TrWxpniNkWeF | pump-fun |
| 4xQ3zWx7gdixiCNCuvvvmZhKnYzAzoq9LxxhezMHFJGK | pump-fun |
| 5AFbwVSQPJf4cgEjq4TajZYyPYUvYktrRN3SahNrPnmv | meteora |
| 5Xv8c46pWRbxtYLXiiVwt9uBkjHHjLU9gNXCggcAgd5c | pump-fun |
| 5Y8T9Q2c3zfYjx2d3zYmMouSGm5mS7cmGrAHKwqAkRax | meteora |
| 5wvUAoDRd2vyKNCZfNuuZLJgx7UHNHP5ueuQ4FLZMCfU | meteora |
| 5yaSS3xHBbRb7crYxEfpC1HosqZrkWiAh7ssn3WFw5bL | meteora-damm-v2 |
| 66RQhRxPBFfFkmcM47UrA6U5T4qcnuVm5VaYjz3Gq9Ed | pump-fun |
| 6BNptAEKGETQFGQgDWiDhFqAgAmHusyqXqJnLLw74Exw | pumpswap |
| 6FLgJdvBJ9hcimhQ4CXZf6JhDnoRzQq9PRnnrqv58vKi | pumpfun |
| 6H9LwRBATqsFTXtsiMmZhwncpXTezk1KLSJ4BC7jz1NU | raydium |
| 6HHjmQjEdTGJ4vZ6Z84bWiLiRUkJLsGqpTQCTrwvyhLh | meteora |
| 6e4e8xVP95GXPCjCyBbRFk7bf96NKoKbXqoQoqX6oHLK | pumpswap |
| 6hYFMyE2BYYScTErqUeCKhihQ5X9j2WD2z9waTdwy38N | raydium |
| 6tBxZ87XSTekESBk1mByNDuKHgxWkfwft8At1ABhtYdg | pump-fun |
| 6voh3PbUKcJXX1ZTyBbqg4sLuiD7N7mtE8TFTGxtgPwf | meteora-damm-v2 |
| 76dpWgLDW4gh4N8bTC7HGooUzeMwUpkZuvRExjBteazG | pumpswap |
| 7RzbtaqfcZJxCsy9ENPLLABv14z7BmQqfafTtayYYKSD | pumpswap |
| 7SKzDTTDK8r8TBrQSfZZLfLqLTYzNBVjHqZF1KL9epsF | meteora-damm-v2 |
| 7TGMXbycXykwz291ZGG3b4Uhynm9akyPpwNPNkrvULyK | pumpfun |
| 7c9pLwwzWRwQRehN7CTMZdhTHnrEo21uM5JDe6ybfaM6 | pumpswap |
| 7dnn4pqWDtQKSUgVcVrvEwiioMvDWSMs8DmSCLkEHHh2 | pump-fun |
| 7j13CapqLC4dGaGYFCxgL8t31R7B1Gt93uBh8EdoRp9H | meteora |
| 7nB4c9xAHHeTCJZxThT8s737N4TBd41ayqLs4obbyCyj | meteora |
| 7xLHeLemYFK4ZLLQcqq6QEum53RD7XedTwJSYsc44e3i | pump-fun |
| 7yrvCBqn4NeowXsxxQECo1WyHpN3gJ4tiaQFCKmv1AHu | raydium-launchlab |
| 81jgrpTMsQWc1rGWXutyYWW27ruiAtehUZwLydcHHWDB | pumpswap |
| 836qAdRaxukxFvPgFqkZQoiDnE9HJKoXtYdCub7imwPG | raydium-launchlab |
| 8JrNUcUimVFPtcY4rdwDwBPiLMiuUWV3nwR5hkHQfhGj | meteora-damm-v2 |
| 8V2h9irZ14XiQSz4dYCQJSbrpvGXvR9iwrLQGqeLghSF | pump-fun |
| 8YSg6aDanXGny6DwaqhWaeBVGCXsFgrUWcLueHpC1auP | pump-fun |
| 8hWJA6TiKrLocY86m9bCZUco5DkxbgGFWmA2mSixVJr3 | raydium-launchlab |
| 8igrMuAMtsRsaex3kBnKXDsojWhPhDPND6nJQJ969GnA | meteora-damm-v2 |
| 8kXZr2BikmKCgzhP4SrLGyhBpSqQWZK2eCHQhxqRD589 | meteora |
| 8q2jaCSxrbD98BBC9M8TsWY8euKCjuhk7hdv7YqJV174 | pumpswap |
| 8uyRGi9cqhQo532vistJLwzGaEtoBwbBoaKKzmJWz1qd | pump-fun |
| 8y1b9Ef7GogpXyGPLG4inkmut8VARCJfiTpVqHf2Lnad | meteora |
| 9FF4a6QCkxyC3t8DW2TUehnUVy9BLVsSPxu8BVFuftT9 | pumpswap |
| 9PgmYzS4wTWnN9jRf7ymxirTxVuqXoSv2bnnGa5LS1XS | meteora-damm-v2 |
| 9Qqd4sN5kVAuR92jAP6FYbJsfDH4hJC6MFxegyfPBHWy | pump-fun |
| 9o9nsuDbnn1N2xJzaCNhezH2iG44KgwHHzXVDTyNLTpn | pump-fun |
| 9wzRduWt7LBSET95rU17ENFgKiUfsBWoLSkHL9aWAWci | meteora-damm-v2 |
| 9yxnwRnYQa2uvjfADuxisQMopmi9YjPMmqrj3JYjcNnq | meteora |
| A7RPpMrJYVNeAGkLAfEMJrkHbFiXtYFq3cdwWRNKu6MU | raydium-launchlab |
| AAt9reFYsz4MLtBKCZJT4koiNZzKGXtGSzMGxNfdZDbF | pump-fun |
| ABF43sbqjwKF1GzY8MkHNHR8EJLZgf6V9ASR8LRmbuCj | pumpswap |
| ABdgCT59RVBbkBsjj73oYAYxJQ8QiJvqHDTAT1jZrttz | pump-fun |
| ADiNaiFFCGitZpMsrAkygarD3YXchPxRQiU4iRSYa1Hy | raydium-launchlab |
| ANg54EGrb7K3yZw9sLhPVJhdaixmuqScxyX5AHcrBnp4 | meteora |
| AZPNXL8oC6ZAwVEFndH3GFzTvRCpzhLZm6YzYQz6j1h8 | pumpfun |
| AbeqLWaD4nQi37xvTZcPUo953cHcLqBUCUYC8MHDP4EQ | pumpswap |
| AnHkAUnHR84zHkgbfHaLsRvGtzev5uDcN4jiU82w5GAP | raydium |
| ArYtLAprgHtsPXSJGS8q9Mw7om8MF5Xdh8QFzLQkUHLi | raydium |
| B7EVmVDKt3VPKsHmG1mjMcD8V2w2JJ5ReiQ6F38aG7ex | pump-fun |
| B9eGWAFCWxcvSc4xtesvE3SibNjHYRUPEQ4S9eyUsNoN | pump-fun |
| BBB5sETGfjaxeVV1r9D73hbasF2T6SeLAnoiRfEYUsjK | raydium-launchlab |
| BGB1po63Kc88xzZQfEy3BBsYAgMHhvHdLhF6DiLndmw3 | pump-fun |
| BRBZpCTppk2zxKHcEXPJbMjYbuZsQaYSGNTuVzVWz5gc | meteora-damm-v2 |
| BUw75VUY7WtFBKwtENh5Q63nkCR4Ft5jFHN6zP6C3ZEs | pump-fun |
| BV22XkXPwNAjWGwi7G8JGr1r3qoyEDZ67z6fg3SPPndC | pump-fun |
| BWNkpvJVuTRfosFdQeihcNqLSsjc9FNrfEcDgxBMY9r5 | pump-fun |
| BY7wyMiM6cCKSfHExTunKeC7LhU2mTgtFBUjMMJqRHXS | meteora-damm-v2 |
| Bs4yPV3sUgdVod8NuTZhaJ5QRLiU8GVrvZZuy41Mg7qy | pumpswap |
| BzomHwB33RGwuyWdm8sQFi3ZDNaUWnpmHRaiQaLQNZiA | pumpswap |
| CHFsqZk6UjHQKidXbEcsJtdgnzjiqpvmRzwHsyNoqJCQ | meteora |
| CK2ypep5JkshFLzwg6TG6v6nz7xT1BHiXa6sYM5bWS6m | meteora |
| CMnb4rm23jxHds46pdjJ1kr2s8DsJP8G2mhkwb2P8fdx | pump-fun |
| CPeAYsUKkkpdPigUz7KHcygDsMrDpYtZAKzu9U2BWyiA | raydium-launchlab |
| CYksJjJfPzb7hn7M6i7BEax2p6ukgS8csSy8UV9X5u1 | pumpswap |
| CnGpHrpgnoeh37YakXvYipfgsvMuU2fcKKtLYEsSDQqc | pumpswap |
| DEXTCER4k9dwbxTMU8zNVVwGevVmvHMB183pF2xxZwvH | pumpswap |
| DLHT6uDwRH4BC4W2VyKqxx3EwaGWLBb7pX8GvrdKmGUk | pumpswap |
| DR6ufMYXncJzmpQHXYyR75Pqyi4EtE6osBd8RBr8g9zW | pumpswap |
| DU8dVHx4YxuMUdt3KByRv8fiuoMG1snmZk6r8qgvQckE | meteora-damm-v2 |
| DeeZoxEwFyN3wsZRPUYA361fxJzPERBYjh2B4iUZEKa7 | pumpswap |
| DikMcqHBoTxUgDVonWZRQj63GU3GVZSbUx2aMziUYxaS | meteora-damm-v2 |
| Djyqwr1pXCbDWBhdQfCaun47oSDTvtkmtCaKcoP5L4LY | raydium-launchlab |
| Dmf2nVvLCz4VxwcP3duEfSGQQBGytLzSVmoohWv3JCUq | pump-fun |
| DnYCUT8hJnsHH3FabqevEqePQkinee8sYeUfA7eiJRin | raydium-launchlab |
| DsQkwmPZipB6qZnXFuUfv1BnCUSSgWQTs4q6FTfnDKs5 | pumpfun |
| DzyuZUVtvvwY1zjpZYuT3Tqu2L6ML17mTiMFf5QrRySd | pumpswap |
| E2hKuDVw626UqaLXqZTwBQMAknTNAg9xsZ6WHkErjMLV | meteora-damm-v2 |
| EEreiJPDJFko67kiVvYBMCurMzV2oyNGLrGJZxziNpTw | raydium-launchlab |
| ETrjD1EayCmBJff2bQTMJEuQ3euKDMkscvGGzdWkXDoy | pump-fun |
| EgXukkEw21Cy3GVNv51Cxb3v5Z6dr3FEZXrW2weWshhA | pumpfun |
| EmnVD6roAkfQFSkN6mtSEQYqKPNvLrYetSfQeqK81EP3 | meteora-damm-v2 |
| EtnDwzL6kMorisRG2QgwkDgVwu5pM5WMfEuN9mca536g | pumpfun |
| EwZxoy5QpQBxo13AHp9fvh11f49CanyFJXXmpSjVtpbP | raydium-launchlab |
| ExcWLAPdv348anqAkjWS4kBo5fSjN4VEdS7qpCpXYV4P | meteora |
| Eyi8QWE7TfLhwRGAJD88KUZpxuxETf9B5HN9ZByVKxi6 | pumpswap |
| F4w8w8QW3K1Srv6pNk2P5WGSgAsHzi4d7VAbt82yoUa6 | meteora-damm-v2 |
| F7J2Mpnh3sbbqiYKmf1cHUFVrxjGWZZcAc6dQSjXUgCr | meteora |
| FEJsEq9mo3myRyU6D3LHz3TXwoE8GrYhEkzkoqyGsKLP | pumpswap |
| FJk9sajdc2Q4heRPPu3yLzkmf2ZByZkpiHQLv9e2bn5A | raydium |
| FKV6jFiRuMmRSSkSDY13Gmwa9yDmNmJ2f76Q4YxkbQw2 | pumpswap |
| FPQyTq1CgSFnGXPBtXoaumcRHXzVfxNaB6GiwNpCZ8qh | meteora-damm-v2 |
| FRHpfqhtx1boJygyg8k6JfeJyFXPbUyH8gn8614svdik | pumpfun |
| FfPawx5sgVecJPhMywfp8uR6Kkf2Kb3vWv18jvCCjz8Q | raydium-launchlab |
| FscgzeeaoKVrBAhoj83gDsLAHW8Azn7LbtG5f9YvdD6s | meteora |
| FwhBB8fh8xDrPEDjzsdaVKsU3175JUpXpJbY2ZnKdtVp | pump-fun |
| G1953X51CbrFqHDG7oZvcGkpoos72A4cPKpJU1fEnSL8 | pump-fun |
| GCApjhJr9QQBCTvQv1brNMy6mh3hdRXu6kvBvwWmZoLM | pump-fun |
| GJd9QRQrY7sbkXVf8N5eZ8fnEVaX3xDdy6jMSKbUpyBg | pump-fun |
| GMrTEw3VbJLneLAZNvo4jXkcyAazfVqfrK8zw4ard6gv | meteora |
| GqnmVjk99nDQVMpHrCuwU5wCnwRe8J3d8vD6WCToBwAb | raydium-launchlab |
| GubXtnTQMKMaTEoY7st8jUgpCEJzkCparDAdGbanBuGa | raydium-launchlab |
| GwsDK6g6LdFZVYyu1abwQhzqBv2WMWm2awE2TXK5hya9 | raydium |
| H25nvp87VPHmuUN8UqH33eipvFfkPxGmHxCCYqZbpXVH | meteora |
| H2pnengTU7Za9kSiSRVJfJx9fFF3qtsa4YuCvJzpVf49 | pumpfun |
| H4sb6cH4bJ4G9vJG3cDA2h3gYAMgCfuzXu8h56RhkVnC | raydium |
| HJa61kEfSAwRt3DW9p15mLh6U5MaQgZGbE51ngg7bYtT | pumpswap |
| HRs3f7XbKEdocBrNzYb8MLmPmDWuVYcwfFe2LjyFcVYb | pump-fun |
| HW7oJPRGwWxiPKtTkGrmDN8isrruEHZN4nEMVT1VoHq3 | pump-fun |
| Hr9qopJXCuak38rxDpZduUFhdAK2BfjtoXbfQZVn2EwU | pumpswap |
| Ht4GxW1pL2SfamcfC3qcZEcjDHpE55TtjUjnKVs2nhxr | pumpswap |
| HwnEx3u2h4aTSNGvVEruyCpFpeQh6LDQ7GM1rHt3bnF1 | raydium-launchlab |
| Hy8daGcCswPCiDEFP1acx5Bv62H1JddvJhKjc2kYPH5D | pumpfun |
| Hy9Ynxf6qwWKYwVcnAv87D9BwqHp4FTEhH6gWjjq86E2 | pump-fun |
| JCXWfcZGjgJC8NZa5Qb6JP3cgvqW4vGUg4Tqj87G5ZnY | raydium |
| Knnmd8dhsfTbZLsPhRyneUQt3ceNs9d8yDWq4LiW9BU | meteora-damm-v2 |
| Q3EWtCYLor6jNWun2fFPf18HG4TJqz6ZbnZ1xYQNNzY | pumpfun |
| Rwq7WrFAzyBtDzGBxYKEHAtHfhAedW6vYqghPT1xBau | raydium |
| TT7DKs9YjJZj1fwMGoPmKBmDreZzyMAMrHoDCGdfwqN | pumpswap |
| eEJ4ZYpgiNTCwRq4cVBdUbqq7s45KrM7XaEFcu3uuGb | pumpswap |
| wXq7Zhb6kbM62ecp7Hnjouctc8kKCPExsMJBUZvFjr1 | meteora-damm-v2 |
| x5Q81ZqSU9jbjmkdbJkY4zaZ8eUANojBz29nxPDRqwD | pumpswap |

## Replay, storage and rendering

Offline replay reconstructs reported pools and three sample rounds from saved raw HTTP responses, and reuses saved RPC account bytes for mint and depth checks. It produced byte-identical output on two invocations. SHA-256: `933d0f01be6016a9a4a9ca1eae43ec49e8612986c35ecccd9dcdf3a1c04bf520`. Initial derived files remain outside Git.

Transactional loading twice left 1 run, 522 pool rows, 354 requirement rows and 18 clue rows. All rows carry run id and evidence kind. Unknown values remain JSON null, shown as Not measured. Money is Decimal, serialized as exact strings.

Headless Chrome rendered the provisioned dashboard. Header shows the required sample caveat. Run health shows 67 public requests, two RPC calls and partial sources. Funnel shows all five steps and actual first-failure reasons. Candidates shows all nine rows through a scrollable table with every clue. Control shows all 13 rows including failures. NewPool exposes all 152 rows through scrolling. The tables use neutral colors, no zero substitution, and no judgment wording. Screenshot: `dashboard-final.png` outside Git.

## Limitations and validation

One short sample cannot establish arbitrage, executable gaps, persistence, or token safety beyond the specified mint checks. Public prices may lag. Discovery is biased by top pools, search, profiles and source availability. Reported price disagreement is only a clue. Unknown combined fees stay visible and must be resolved before further interpretation. No thresholds were tuned after collection. No wallet, signing, submission, continuous stream or unattended retry path was added.

Validation: pinned Cargo 1.92.0-nightly; offline helper build passed; 9 Python test groups passed (synthetic mint fixtures, exact boundaries, unknowns, first failure, depth, limits, 429 retry cap, determinism and source-path check); existing gap-survey package fmt/clippy passed and 13 tests passed. Both dashboard checks, strict OpenSpec validation, repo hygiene and diff whitespace validation passed. Existing repo hygiene documentation warnings remain. The new implementation extends existing scripts, so no new Rust crate was needed. Protected crates, Handoff 18 results and seed data are unchanged. The nonprinting credential scan and final process/volume checks are recorded at completion.

Completion checks: strict OpenSpec validation passed 14/14 after syncing and archiving `2026-10-31-pool-map-sample`. Nonprinting credential scan checked 18 changed files/manifests with zero pattern hits; no raw response or environment file is committed. No protected crate or Handoff 18 data file changed. The collector, account helper and temporary browser process have exited. The research containers are stopped and both research Docker volumes remain. No merge was performed.
