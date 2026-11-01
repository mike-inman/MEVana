# 2026-10-31 23:42 CDT — Pool map sample plan

Created: 2026-10-31 23:42 CDT

Authority: handoff 19. Base is local main c7c5d34a, containing c6aa0175. Extend the standard-library research scripts instead of adding a Rust crate; reuse the existing Tatum public-account helper and its named credential loader. No Handoff 18 source, result or seed file changes.

Public documentation checked before collection:
- GeckoTerminal: https://api.geckoterminal.com/docs/index.html; top pools by exchange and new Solana pools. Public limit 30 calls/minute: https://support.coingecko.com/articles/6867086195-does-geckoterminal-have-an-api . Pace at 2.2 seconds/request; cap retries at two after the original call.
- DexScreener: https://docs.dexscreener.com/api/reference ; search, recent token profiles, token pools and batched pair lookup. Pair endpoints allow 300/minute; recent profiles allow 60/minute. Pace all requests at 1.1 seconds or slower. DexScreener has no documented top-pools-by-exchange endpoint, so its discovery component is explicitly a search/profile sample, not a complete top list.

Freeze R1 two pools, R2 reported liquidity 20000, R3 seven days, R4 mint restrictions with named USDC/USDT issuer-freeze exemptions, R5 gap greater than combined fees; unknown R5 remains review. Shortlist at most 40 pairs, prioritizing controls then reported liquidity, at most three pools each. Collect three price samples five minutes apart. A shared 1190-second deadline, 300-request ceiling and 20-call RPC ceiling stop projected overruns. Raw responses and replay inputs remain outside Git. Candidate ordering is lowest turnover above median liquidity. Comparison sampling uses SHA-256 of run id and excludes selected candidates.
