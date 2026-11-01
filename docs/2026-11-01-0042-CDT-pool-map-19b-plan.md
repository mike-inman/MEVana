# 2026-11-01 00:42 CDT — Check every eligible saved-map pair

Created: 2026-11-01 00:42 CDT

Authority: Handoff 19b. Reuse the saved 19a map with run id sample-20261101-19b. The offline plan selects 91 pairs, 239 pools and 55 mints without trimming. Its conservative bound is 13 RPC calls (one config per pool assumed) and 24 normal public price requests. Stop before live use if a plan exceeds 15 calls. Read all mints once, then three pool rounds 120 seconds apart. Public calls are batched reported prices only; enforce 40 calls, 15 Tatum calls and a 460-second guard. Preserve both older runs. Keep every numeric threshold and list rule unchanged. NotChecked is distinct from rejected unreadable state. The nullable within-bin label does not participate in selection.
