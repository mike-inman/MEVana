# 2026-10-31 CDT — Indicative price-gap survey

This crate records and analyzes public pool account mid prices only. A price gap is Indicative evidence, not an executable quote or profit.

## Pinned layouts

- Orca Whirlpool: `whirlpool-core::decode`, 653-byte account and the repository's existing pinned Whirlpool source.
- Raydium CLMM: official `raydium-idl/raydium_clmm/raydium_clmm.json` at commit `e7e0c96fe77bcf6a020b84a44c47a722aac8e359`, IDL version 0.1.0; `PoolState` is packed and 1,536 bytes after the discriminator, `AmmConfig` is 109 bytes after the discriminator. Published program id ends `WqK`; the older `shared-core` constant does not match this program and is not used here.
- Meteora DLMM: official `MeteoraAg/dlmm-sdk/idls/dlmm.json` at commit `2f540c2107988e0f97efa5917100bb912c96c17a`, IDL version 0.12.0; `LbPair` is C-layout and 896 bytes after the discriminator. Published program id ends `Pwxo`; the older `shared-core` constant is not used here.

The source links are https://github.com/raydium-io/raydium-idl/blob/e7e0c96fe77bcf6a020b84a44c47a722aac8e359/raydium_clmm/raydium_clmm.json and https://github.com/MeteoraAg/dlmm-sdk/blob/2f540c2107988e0f97efa5917100bb912c96c17a/idls/dlmm.json. The checked-in registry's file named `meteora_dlmm.json` identifies `amm_v3` and is not used.

Meteora's variable fee is not modeled. Any net gap involving it remains unknown unless the full fee is known. Raydium's dynamic fee is likewise treated as unknown when nonzero configuration/state is present.
