# Golden Whirlpool quote cases — 2026-10-29 CDT

`whirlpool-v1.jsonl.zst` is a bounded subset of real public on-chain data from the six finalized handoff 09 captures. It is quote-correctness evidence, not evidence of executable or profitable opportunities. The full version 1 corpus and classification report remain outside Git at `~/MEVanaData/golden/whirlpool-20261029/`.

Each JSONL row has `version: 1`, base58 pool and signature identifiers, slot, direction, observed vault input/output atoms, raw pre/post pool account byte arrays, raw keyed tick-array byte arrays, quote results and an initialized-tick-crossing flag. The archive is zstd compressed. The regression test re-decodes account bytes and requires one quote mode to match both observed vault amounts exactly for every row.
