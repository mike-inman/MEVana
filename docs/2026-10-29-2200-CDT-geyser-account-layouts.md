# 2026-10-29 22:00 CDT — Handoff 09 account layout derivation

No provider calls were used. Local `idls/whirlpool_idl.json` supplies Whirlpool, Tick, TickArray and Position fields. Current Orca source at commit `cc683ec92acb5e7606caa3d8e514d9383e9c7c11` supplies DynamicTickArray and Oracle:
https://github.com/orca-so/whirlpools/tree/cc683ec92acb5e7606caa3d8e514d9383e9c7c11/programs/whirlpool/src/state

Discriminators below are the first eight SHA-256 bytes of `account:<Name>` (Anchor). This identifies the source layout expected for each observed size family; capture inspection must still confirm actual bytes by discriminator, never length alone.

| Family | Discriminator hex | Size including discriminator | Pool memcmp offset |
|---|---|---|---|
| Whirlpool | 3f95d10ce1806309 | 8+261+3×128 = 653 | n/a |
| TickArray (fixed) | 4561bdbe6e0742bb | 8+4+88×113+32 = 9988 | 8+4+88×113 = 9956 |
| DynamicTickArray | 11d8f68ee1c7da38 | 8+4+32+16+88+112×n = 148+112×n, n=0..88 | 8+4 = 12 |
| Position | aabc8fe47a40f7d0 | 8+2×32+16+2×4+2×(16+8)+3×(16+8) = 216 | excluded |
| Oracle | 8bc283b38cb3e5f4 | 8+32+8+34+44+128 = 254 | excluded |

Tick is bool plus four 16-byte scalars and three 16-byte rewards: 1+64+48=113. DynamicTick is a one-byte enum tag plus 112 bytes only when initialized. A dynamic array holds start i32, pool Pubkey, u128 bitmap and 88 dynamic ticks. Its full range is 148..10004 bytes. The older IDL does not define it.

Oracle's AdaptiveFeeConstants fields sum to five u16 + two u32 + 16 reserved = 34. AdaptiveFeeVariables sum to two u64 + three u32 + 16 reserved = 44. The remaining Oracle fields are pool Pubkey, trade-enable u64 and 128 reserved bytes. This accounts for the previously unidentified 254-byte family; it is neither a position nor a tick array.

Pool layout offsets from the IDL: tick spacing 41, fee rate 45, mint A 101, mint B 181; reward infos begin at 269, three 128-byte structs end at 653. `WhirlpoolMinimal` agrees (its old reward comment was wrong). Fixed and dynamic filters include discriminator memcmp at offset zero. Pool filters include both datasize and discriminator. Position and Oracle accounts are excluded.

Dynamic arrays are captured and counted, but decoding their ticks is unsupported in this slice. No arithmetic/quoter changes are made. The code's layout tests independently walk the local IDL and verify all fixed sizes/offsets; source-derived dynamic/Oracle constants are checked against the formulas above.
