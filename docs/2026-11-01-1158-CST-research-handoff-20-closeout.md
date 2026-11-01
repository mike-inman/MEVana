# 2026-11-01 12:01 CST — Handoff 20 reachable-gap closeout

Created: 2026-11-01T12:01:44-06:00

Authority: `2026-11-01-1131-CST-research-handoff-20-meteora-reachable-gaps.md`. Base local main `a8eb0e28`; branch `feat/meteora-reachable-gaps-20261101`. No merge.

## Result

**Zero non-control pairs are Reachable.** The fully modeled Meteora routes are NotReachable at $100 and $1,000 in both pool orders, in both rounds. WBTC/USDC uses an Orca leg and remains Indicative, so the exact Orca question is unresolved. Zero is a result for this bounded sample and model, not proof that no market opportunity exists.

The nine canonical pairs produce 11 qualifying pool comparisons and 44 size/direction summaries: 28 NotReachable and 16 Indicative. All 88 individual round observations have those same respective labels. There are no Not measured rows in the completed sample.

## Collection and fixed plan

Run: `sample-20261101-20` under `~/MEVanaData/pool-map/`. Exactly one collection: **8 Tatum getMultipleAccounts calls**, **0 public market API requests**, **0 Geyser bytes**, **129.964 seconds**. Pool-state rounds were exactly 120 seconds apart. Each round reads 21 pool accounts and 10 mint accounts, then 205 bin arrays in three batches. The conservative preflight bound was 10 calls (16 Meteora pools × at most 25 arrays; batched by 100), below the hard 15-call ceiling. Collection subprocesses have an 8-second bound and an overall 280-second read safety limit. No retries or repeat collection occurred.

The plan was frozen before reading live data in `docs/2026-11-01-1144-CST-handoff-20-account-plan.json`. Pool comparisons came from saved 19b eligible pools clearing their own minimum combined fee in any round; BP/USDC was excluded by R4. Sizes remained $100/$1,000. Reachable remained strict output atoms > input atoms, complete modeled legs in one round, and slot spread ≤25. Maximum observed spread was 7 slots.

Batched state includes the active bin array and up to 12 initialized arrays on each side. A clear internal bitmap bit proves an uninitialized array empty; an unread initialized array is unknown. Bitmap extensions and unmodeled liquidity features fail closed. The sampled arrays reached liquidity on both sides of every Meteora pool. Missing coverage would be Not measured, never empty.

USD valuation is an explicit model input: USDC/USDT assumed $1; SOL and JUP use the following same-round nominal reference pools. References are price inputs only, not extra route legs. The fund asset is the stablecoin when present, otherwise SOL, otherwise JUP. Atom inputs are rounded down from the nominal dollar size.

- Funding mint `JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN`: reference pool `BhQEFZCRnWKQ21LEt4DUby7fKynfmLVJcNjfHNqjEF61`.
- Funding mint `So11111111111111111111111111111111111111112`: reference pool `7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm`.

No Orca tick arrays were part of the frozen plan. Orca legs use clearly labeled nominal standard-fee estimates with unmeasured impact; **they can never be Reachable**. This conserves the bounded account plan but limits WBTC/USDC and three control comparisons. Before transaction fees, priority fees and tips; no transaction construction or execution was performed.

## Verified decoder and model

Published IDL: [lb_clmm 0.12.0 (spec 0.1.0)](https://github.com/MeteoraAg/dlmm-sdk/blob/2f540c2107988e0f97efa5917100bb912c96c17a/idls/dlmm.json). Full upstream IDL SHA-256: `045c0f4af044be046b6a14b350077e4242e8ac026b64a855803dab30fcdd8b35`. The committed type subset retains the original fields, padding, account discriminators and source digest. All used offsets and array strides are derived from these types with Solana C alignment, rather than guessed from a binary. Real-account tests check LbPair = 904 bytes, BinArray = 10,136 bytes, Bin = 144 bytes; active ID at 76, bin step at 80, and bins starting at 56.

**Offset 56 is confirmed:** `LbPair.v_parameters.last_update_timestamp` (8-byte discriminator + 32-byte StaticParameters + 16-byte relative offset). It is the variable-parameter update timestamp. It is not an independently verified timestamp of the last trade. `last_updated_at` elsewhere is the pool fee-parameter update field and is not substituted for it.

IDL and quote behavior were checked against the published SDK at the same commit: `helpers/fee.ts`, `helpers/bin.ts`, `helpers/binArray.ts`, `helpers/derive.ts`, `helpers/lbPair.ts`, constants and the SDK reference/volatility methods. Static published documentation reads are distinct from collection; no public market API was called. The decoder supports enabled, activated permissionless pool types 0, 2 and 3 from the IDL; permissioned/unknown types and unsupported token programs fail closed.

All swap amounts, reserves, fees and outputs use integers. Base fee = base_factor × bin_step × 10 × 10^power; variable fee = ceil(control × (volatility × bin_step)^2 / 100,000,000,000); total is capped at 100,000,000 / 1e9. Reference decay and per-crossed-bin volatility follow the SDK. Input/output fee modes and ceil/floor fill rounding are modeled. Protocol share is already part of the trading fee, not an extra charge. Limit-order liquidity is not silently ignored: nonzero unsupported order reserves cause Not measured. No such condition blocked these quotes.

Real JUP/MET example (fixture pool `3xNGdc58…`): base factor 10,000, bin step 50, fee control 10,000. After the 1,200-second decay, active 18 sets reference 18. The first liquid bin for X→Y is 17: volatility 10,000; base rate 5,000,000; variable rate 25,000; total rate 5,025,000 / 1e9 = **50.25 bps**. Further crossings change the rate. Fixtures are unmodified public account bytes with addresses, slots and hashes in `scripts/research/pool_map/fixtures/h20-public-accounts.json`.

## Every pool comparison, size and direction

Tables retain both rounds; no favorable round hides an unfavorable one. Dollar columns are modeled output minus the actual atom input, expressed through the declared funding valuation, before transaction/priority/tip costs. Average buy/sell prices include trading fees and are funding tokens per intermediate token. Fee columns give each leg’s minimum–maximum total trading rate across the two walks in basis points (Modeled for Meteora, Indicative for Orca). Exact fees in input/output atoms and every bin fill remain in `result.json` and reproduce offline. Display rounding below does not feed the model.

### 1. Bonk / SOL — control

- Pool A: `5zpyutJu9ee6jFymDGoK7F6S5Kczqtc9FomP3ueKuyA9`.
- Pool B: `AtcGozT6i1GR3wJAjVwRwmt7qbHqZrBUqKs2DcMAKoVe`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | Indicative / Indicative | -0.861287 | -0.867793 | 0.000000030860 / 0.000000030594 | 0.000000030862 / 0.000000030594 | Indicative 5.00000–5.00000 | 25.00000–25.00000 |
| 100 | B → A | Indicative / Indicative | -0.735468 | -0.728953 | 0.000000031057 / 0.000000030829 | 0.000000031057 / 0.000000030831 | 25.00000–25.75000 | Indicative 5.00000–5.00000 |
| 1,000 | A → B | Indicative / Indicative | -8.612873 | -8.677929 | 0.000000030860 / 0.000000030594 | 0.000000030862 / 0.000000030594 | Indicative 5.00000–5.00000 | 25.00000–25.00000 |
| 1,000 | B → A | Indicative / Indicative | -7.354676 | -7.289532 | 0.000000031057 / 0.000000030829 | 0.000000031057 / 0.000000030831 | 25.00000–25.75000 | Indicative 5.00000–5.00000 |

### 2. Bonk / USDC — control

- Pool A: `31p1hptjhFo6ZD8oBqkfutNXQKGGPyi7YcEAfsyKW777`.
- Pool B: `8QaXeHBrShJTdtN1rWCccBxpSVvKksQ2PCu5nufb2zbk`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | Indicative / Indicative | -0.676597 | -0.676597 | 0.000002934759 / 0.000002914902 | 0.000002934759 / 0.000002914902 | 20.00000–20.00000 | Indicative 30.00000–30.00000 |
| 100 | B → A | Indicative / Indicative | -1.117740 | -1.117740 | 0.000002932471 / 0.000002899693 | 0.000002932471 / 0.000002899693 | Indicative 30.00000–30.00000 | 20.48000–20.48000 |
| 1,000 | A → B | Indicative / Indicative | -6.765968 | -6.765968 | 0.000002934759 / 0.000002914902 | 0.000002934759 / 0.000002914902 | 20.00000–20.00000 | Indicative 30.00000–30.00000 |
| 1,000 | B → A | Indicative / Indicative | -11.177394 | -11.177394 | 0.000002932471 / 0.000002899693 | 0.000002932471 / 0.000002899693 | Indicative 30.00000–30.00000 | 20.48000–20.48000 |

### 3. HYPE / USDC

- Pool A: `6F4rVnmVc1A2QDqpHn5cpQZfXugapFbGZTXEyaakpvVQ`.
- Pool B: `DXfnX2oCJAcfBC8A7MB1UamcrT9eeERxWP2RduHkrbN`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | NotReachable / NotReachable | -1.099164 | -1.099046 | 88.232573440294 / 87.262752756765 | 88.232468343095 / 87.262752929069 | 10.00063–10.06250 | 10.00000–10.00000 |
| 100 | B → A | NotReachable / NotReachable | -0.397300 | -0.397282 | 88.318546868523 / 87.967657281815 | 88.318546868523 / 87.967673179153 | 10.75000–10.75000 | 10.00063–10.00250 |
| 1,000 | A → B | NotReachable / NotReachable | -11.444856 | -11.443234 | 88.273025277349 / 87.262753214365 | 88.272880445322 / 87.262753218532 | 10.00063–10.20250 | 10.00000–10.00000 |
| 1,000 | B → A | NotReachable / NotReachable | -4.426528 | -4.425891 | 88.318546845123 / 87.927602324594 | 88.318546845123 / 87.927658583508 | 10.75000–10.75000 | 10.00063–10.06250 |

### 4. HYPE / USDC

- Pool A: `ANCx141SujgVdbKz9NTEH8F38qWsnyyXsVju64aU3qLB`.
- Pool B: `DXfnX2oCJAcfBC8A7MB1UamcrT9eeERxWP2RduHkrbN`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | NotReachable / NotReachable | -1.119915 | -1.119915 | 88.251089790866 / 87.262752598634 | 88.251089790866 / 87.262752598634 | 20.00000–20.00000 | 10.00000–10.00000 |
| 100 | B → A | NotReachable / NotReachable | -0.670852 | -0.670852 | 88.318546868523 / 87.726060130485 | 88.318546868523 / 87.726060130485 | 10.75000–10.75000 | 20.00000–20.08000 |
| 1,000 | A → B | NotReachable / NotReachable | -11.199142 | -11.199142 | 88.251089720771 / 87.262753235334 | 88.251089720771 / 87.262753235334 | 20.00000–20.00000 | 10.00000–10.00000 |
| 1,000 | B → A | NotReachable / NotReachable | -6.746947 | -6.746947 | 88.318546845123 / 87.722666290442 | 88.318546845123 / 87.722666290442 | 10.75000–10.75000 | 20.00000–20.08000 |

### 5. JUP / MET

- Pool A: `3xNGdc58axYtrJ64STQz5TrdQWVtWHLR888iRBbWZnEe`.
- Pool B: `EoFtFBgdBXzKeqZc5dtYGvnKicWEvfEZGTZQM9eFTJ6b`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | NotReachable / NotReachable | -1.131463 | -1.131463 | 1.099426072788 / 1.086986478565 | 1.099426072788 / 1.086986478565 | 50.00000–50.00000 | 10.00000–10.36000 |
| 100 | B → A | NotReachable / NotReachable | -0.771254 | -0.771254 | 1.091434593948 / 1.083016857377 | 1.091434593948 / 1.083016857377 | 10.00000–10.16000 | 50.25000–50.25000 |
| 1,000 | A → B | NotReachable / NotReachable | -21.433504 | -21.433504 | 1.099426070350 / 1.075861516931 | 1.099426070350 / 1.075861516931 | 50.00000–50.00000 | 10.00000–29.36000 |
| 1,000 | B → A | NotReachable / NotReachable | -17.119860 | -17.119860 | 1.101880909401 / 1.083016862093 | 1.101880909401 / 1.083016862093 | 10.00000–26.00000 | 50.25000–50.25000 |

### 6. JUP / SOL — control

- Pool A: `C1MgLojNLWBKADvu9BHdtgzz1oZX4dZ5zGdGcgvvW8Wz`.
- Pool B: `C8Gr6AUuq9hEdSYJzoEpNcdjpojPZwqG5MtQbeouNNwg`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | Indicative / Indicative | -0.147567 | -0.170386 | 0.002729081226 / 0.002725054013 | 0.002729705040 / 0.002725054007 | Indicative 5.00000–5.00000 | 15.00000–15.00000 |
| 100 | B → A | Indicative / Indicative | -0.252257 | -0.229456 | 0.002733247614 / 0.002726352817 | 0.002733247614 / 0.002726976009 | 15.00000–15.00000 | Indicative 5.00002–5.00002 |
| 1,000 | A → B | Indicative / Indicative | -1.475665 | -1.703856 | 0.002729081223 / 0.002725054013 | 0.002729705039 / 0.002725054014 | Indicative 5.00000–5.00000 | 15.00000–15.00000 |
| 1,000 | B → A | Indicative / Indicative | -2.522561 | -2.294557 | 0.002733247608 / 0.002726352823 | 0.002733247608 / 0.002726976015 | 15.00000–15.00000 | Indicative 5.00000–5.00000 |

### 7. JUP / SOL — control

- Pool A: `C8Gr6AUuq9hEdSYJzoEpNcdjpojPZwqG5MtQbeouNNwg`.
- Pool B: `HdsFGjjY46twFKjqHqUyT2bnRS4XCo1HaExts5CSNprU`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | NotReachable / NotReachable | -0.424125 | -0.424125 | 0.002733247614 / 0.002721655241 | 0.002733247614 / 0.002721655241 | 15.00000–15.00000 | 15.00000–15.06750 |
| 100 | B → A | NotReachable / NotReachable | -0.325458 | -0.325458 | 0.002733951862 / 0.002725054007 | 0.002733951862 / 0.002725054007 | 15.00000–15.00000 | 15.00000–15.00000 |
| 1,000 | A → B | NotReachable / NotReachable | -6.125480 | -6.125480 | 0.002733247608 / 0.002716505154 | 0.002733247608 / 0.002716505154 | 15.00000–15.00000 | 15.00000–16.08000 |
| 1,000 | B → A | NotReachable / NotReachable | -5.134789 | -5.134789 | 0.002739118812 / 0.002725054014 | 0.002739118812 / 0.002725054014 | 15.00000–15.60750 | 15.00000–15.00000 |

### 8. PENGU / USDC

- Pool A: `9uX6G8ZaRVxGSVybyYwqjHdgA1hgQqsospqD4m1sG3u2`.
- Pool B: `DdMA1cHcHEqYfttc1z1sJEY978CcU1pyjNuTWTNmdvzU`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | NotReachable / NotReachable | -4.032896 | -4.032896 | 0.007538800390 / 0.007234768411 | 0.007538800390 / 0.007234768411 | 20.00000–49.29688 | 25.00000–25.09375 |
| 100 | B → A | NotReachable / NotReachable | -4.620890 | -4.620890 | 0.007289322875 / 0.006952491283 | 0.007289322875 / 0.006952491283 | 25.00000–25.00000 | 20.00000–77.42188 |
| 1,000 | A → B | NotReachable / NotReachable | -226.256791 | -226.256791 | 0.009350346285 / 0.007234766940 | 0.009350346285 / 0.007234766940 | 20.00000–283.67188 | 25.00000–25.09375 |
| 1,000 | B → A | NotReachable / NotReachable | -96.770052 | -96.770052 | 0.007289322875 / 0.006583934721 | 0.007289322875 / 0.006583934721 | 25.00000–25.00000 | 20.00000–114.92188 |

### 9. USDC / JUP — control

- Pool A: `BhQEFZCRnWKQ21LEt4DUby7fKynfmLVJcNjfHNqjEF61`.
- Pool B: `HfgjZDmexhFVD28Vkb1NbQwWeXP3uDcVTLPjSGHmRHhL`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | NotReachable / NotReachable | -0.519083 | -0.519083 | 0.258616904485 / 0.257274468098 | 0.258616904485 / 0.257274468098 | 10.01231–10.09668 | 10.00000–10.00000 |
| 100 | B → A | NotReachable / NotReachable | -0.471532 | -0.471532 | 0.259085223654 / 0.257863553917 | 0.259085223654 / 0.257863553917 | 10.25000–10.25000 | 10.01231–10.01231 |
| 1,000 | A → B | NotReachable / NotReachable | -58.797351 | -58.797351 | 0.273346519985 / 0.257274468705 | 0.273346519985 / 0.257274468705 | 10.01231–59.00000 | 10.00000–10.00000 |
| 1,000 | B → A | NotReachable / NotReachable | -7.570767 | -7.570767 | 0.259085223386 / 0.257123749526 | 0.259085223386 / 0.257123749526 | 10.25000–10.25000 | 10.01231–11.71852 |

### 10. USDT / FO

- Pool A: `AXXo7N2gcLeVyo8nDsN99UW8Fv6NCsa2DP21WD3SKtM3`.
- Pool B: `EXXfbVutbuexDZtPw8pkSvWxLFoUMBHd544CAGkmtPZm`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | NotReachable / NotReachable | -0.393839 | -0.448059 | 0.055379583882 / 0.055161477483 | 0.055379583882 / 0.055131450672 | 20.00000–20.00000 | 1.00001–1.16267 |
| 100 | B → A | NotReachable / NotReachable | -0.947260 | -0.903656 | 0.055245679348 / 0.054722359126 | 0.055221319195 / 0.054722308431 | 1.00001–1.16251 | 20.00000–20.75000 |
| 1,000 | A → B | NotReachable / NotReachable | -13.601734 | -14.020014 | 0.055566341285 / 0.054810542691 | 0.055566341285 / 0.054787300402 | 20.00000–20.75000 | 1.00001–8.85861 |
| 1,000 | B → A | NotReachable / NotReachable | -24.224478 | -23.525565 | 0.055816134641 / 0.054464017915 | 0.055775882233 / 0.054463723090 | 1.00001–22.18872 | 20.00000–23.00000 |

### 11. WBTC / USDC

- Pool A: `3sehQcVywWcFJZ1ri3NmJ7MRkrXbViJMRNN5b6kz8Mqn`.
- Pool B: `55BrDTCLWayM16GwrMEQU57o4PTm6ceF9wavSdNZcEiy`.

| Size $ | Order | Result R1 / R2 | Net $ R1 | Net $ R2 | Avg buy/sell R1 | Avg buy/sell R2 | Buy fee bps | Sell fee bps |
| ---: | --- | --- | ---: | ---: | --- | --- | --- | --- |
| 100 | A → B | Indicative / Indicative | -0.623180 | -0.621959 | 78396.324780294300 / 77907.774563528462 | 78396.324780294300 / 77908.731782654029 | 10.00000–10.00000 | Indicative 5.01736–5.01736 |
| 100 | B → A | Indicative / Indicative | -0.178877 | -0.180435 | 77986.087282028886 / 77846.588108681411 | 77987.303666963018 / 77846.587275591534 | Indicative 5.00000–5.00000 | 10.00000–10.25000 |
| 1,000 | A → B | Indicative / Indicative | -8.529674 | -8.517487 | 78578.113323783473 / 77907.867633596544 | 78578.113323783473 / 77908.825265063621 | 10.00000–10.25000 | Indicative 5.00543–5.00543 |
| 1,000 | B → A | Indicative / Indicative | -4.151137 | -4.163543 | 77985.904827561467 / 77662.174652553298 | 77986.877927919848 / 77662.176208231203 | Indicative 5.00000–5.00000 | 10.00000–11.00000 |

## Repeated gaps and limits of the explanation

Fifteen of sixteen Meteora active prices did not change across these two rounds. EXXfbVut… (USDT/FO) changed. This is consistent with the earlier nominal-price persistence but cannot establish inactivity by itself. The verified fee-state ages below also do not establish when a trade last happened.

- **PENGU/USDC:** both active bins contain two-sided reserves, but $100 already crosses six bins on one leg; $1,000 crosses more and raises variable fees substantially. The repeated nominal gap is not a $100/$1,000 execution price. Thin liquidity, price impact and rising fees explain the modeled losses; an empty active bin is not the explanation here.
- **HYPE/USDC:** DXfnX2o… has no output-X reserves in the active bin; the nearest output-X liquidity is one bin away. Walking to that liquidity removes the apparent advantage. Fees and size impact remain additional costs.
- **JUP/MET:** 3xNGdc58… has no output-Y reserves at its active bin; nearest output-Y liquidity is one bin lower. $100 also crosses multiple bins in the other pool; $1,000 crosses substantially more. Both the one-sided nominal active price and full fees/impact matter.
- **USDT/FO:** the EXXfbVut… price moved between rounds, so the earlier frozen-gap hypothesis is not universal. Both active bins have reserves, but sizes walk across bins and variable fees increase; neither order clears costs.
- **WBTC/USDC:** one Meteora output-X active reserve is effectively dust (two input atoms are enough to exhaust it in the indicative best order); the route walks beyond the nominal price. Both nominal round trips are negative, but exact Orca impact remains unmeasured. Its exact outcome and the historical cause of its earlier gap remain unknown.
- **Controls:** Bonk/USDC has an empty output-Y active side; USDC/JUP has an empty output-X active side in HfgjZDme…. Other controls demonstrate thin or dust reserves and fee/impact costs even when a bin is technically nonempty. Three control comparisons include Indicative Orca legs.

These are explanations from observed reserve walks, not a causal decomposition into counterfactual profits. We did not alter sizes, rules or fees to fit the outcome. A two-minute sample cannot distinguish all historical causes of repeated gaps, and it does not establish future persistence or executable profit.

## Meteora labels (second round)

Empty-to-Y walks downward from the active bin (X input). Empty-to-X walks upward (Y input). Zero means reserves exist in the active bin, not that they have useful size. Ages are seconds since the verified variable-parameter update timestamp.

| Full pool | Active | Changed | Empty to Y | Empty to X | Fee-state age s |
| --- | ---: | --- | ---: | ---: | ---: |
| `31p1hptjhFo6ZD8oBqkfutNXQKGGPyi7YcEAfsyKW777` | -1310 | False | 1 | 0 | 1433 |
| `3sehQcVywWcFJZ1ri3NmJ7MRkrXbViJMRNN5b6kz8Mqn` | 1336 | False | 0 | 0 | 3360 |
| `3xNGdc58axYtrJ64STQz5TrdQWVtWHLR888iRBbWZnEe` | 18 | False | 1 | 0 | 19929 |
| `6F4rVnmVc1A2QDqpHn5cpQZfXugapFbGZTXEyaakpvVQ` | -2431 | False | 0 | 0 | 578 |
| `9uX6G8ZaRVxGSVybyYwqjHdgA1hgQqsospqD4m1sG3u2` | -396 | False | 0 | 0 | 22953 |
| `ANCx141SujgVdbKz9NTEH8F38qWsnyyXsVju64aU3qLB` | -1216 | False | 0 | 0 | 875 |
| `AXXo7N2gcLeVyo8nDsN99UW8Fv6NCsa2DP21WD3SKtM3` | -291 | False | 0 | 0 | 13941 |
| `AtcGozT6i1GR3wJAjVwRwmt7qbHqZrBUqKs2DcMAKoVe` | -813 | False | 0 | 0 | 6762 |
| `BhQEFZCRnWKQ21LEt4DUby7fKynfmLVJcNjfHNqjEF61` | -1355 | False | 0 | 0 | 279 |
| `C8Gr6AUuq9hEdSYJzoEpNcdjpojPZwqG5MtQbeouNNwg` | 126 | False | 0 | 0 | 6142 |
| `DXfnX2oCJAcfBC8A7MB1UamcrT9eeERxWP2RduHkrbN` | -245 | False | 0 | 1 | 16424 |
| `DdMA1cHcHEqYfttc1z1sJEY978CcU1pyjNuTWTNmdvzU` | -1972 | False | 0 | 0 | 3056 |
| `EXXfbVutbuexDZtPw8pkSvWxLFoUMBHd544CAGkmtPZm` | -2899 | True | 0 | 0 | 48 |
| `EoFtFBgdBXzKeqZc5dtYGvnKicWEvfEZGTZQM9eFTJ6b` | -86 | False | 0 | 0 | 4017 |
| `HdsFGjjY46twFKjqHqUyT2bnRS4XCo1HaExts5CSNprU` | 670 | False | 0 | 0 | 10346 |
| `HfgjZDmexhFVD28Vkb1NbQwWeXP3uDcVTLPjSGHmRHhL` | -272 | False | 0 | 1 | 6280 |

## R5 and preservation

R5 now chooses from `passes` when any eligible pool comparison clears its own combined fee. A synthetic regression proves that a smaller cheap gap passes even if the largest expensive gap fails. Offline rerun of saved 19b produced **0 changed pairs out of 796 rows**. All **51 saved 19b files are byte-identical** to pre-work hashes. Earlier sample directories were not written. Local `infrastructure/research.env` is unchanged. Protected crates, UI and main Docker compose were not edited.

## Dashboard and stored evidence

Browser rendering used headless Chrome at 1500 pixels wide because the browser access host was unavailable. Both new panels use neutral colors and explicit evidence labels:

- **Reachable gap test — Modeled / Indicative:** 44 rows, one per full pool comparison, size and order. Shows 28 NotReachable / 16 Indicative, dollar deltas and fee-inclusive average prices. Zero Reachable. The summary selects the most favorable complete modeled round when available; both rounds remain in storage and above. Tiny prices have twelve displayed decimals; missing-state reasons stay unknown, while no reason displays blank.
- **Meteora liquidity and fee-state labels — OnChain:** 16 rows with active bins, price-change flag, fee-state age, nearest output-side empty-bin distances, slots and full pool identifiers. Fifteen unchanged, one changed. All nearest-liquidity distances were measured (0 or 1).
- Existing tables retain their evidence and data. Candidate tables fit all 8 rows, comparisons all 6 and controls all 13 without internal row scrolling. The NewPool table has room for all 505 saved rows. SQL returns a true empty first-failure value when nothing failed; a field-specific blank display prevents Grafana from turning it into Not measured. The 1500-pixel control view verifies the blank cells and all 13 controls.

Postgres queries returned `Indicative|16`, `NotReachable|28`, 16 pool labels, and **0** non-control Reachable rows. New tables are separate from earlier pool-map records. Full raw account data remains outside Git, with only two small public account fixtures committed.

## Determinism and validation

Result and repeated offline replay SHA-256: `d9281f9c89bc079444b25a7395c66c2a2b08517eb4b0aa842cf813952f193ac9`. Both files were byte-identical. The saved-run replay test additionally compares regenerated canonical bytes to the stored result.

```text
cargo 1.92.0-nightly (24bb93c38 2025-09-10)
cargo test -p gap-survey --offline: 13 unit + 1 integration passed
cargo fmt -p gap-survey --check: passed
cargo clippy -p gap-survey --offline --all-targets -- -D warnings: passed
python3 -m unittest discover -s scripts/research/pool_map -p test_*.py: 30 passed
check-dashboard.py: existing evidence guards + reachable-gap checks passed
check-gap-dashboard.py: 8-panel contract passed
openspec validate --all --strict: passed
repo_hygiene.sh: passed (existing documentation/archive-script warnings)
git diff --check: passed
credential scan: passed, no secrets printed
```

The existing HTTP-429 mock test emits a Python ResourceWarning while all tests pass. Three new saved-data acceptance tests skip on machines where their local saved data is not installed; all three ran here. No new dependencies were added. The OpenSpec change is archived with the synced requirements. The branch is committed and pushed without merging. Research containers are stopped with both persistent volumes retained; collection processes and temporary browser sessions have exited.
