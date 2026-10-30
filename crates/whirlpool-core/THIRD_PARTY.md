# Third-party dependencies — 2026-10-29 CDT

`orca_whirlpools_core` and `orca_whirlpools_macros` are pinned to exactly 1.0.4, the 2025-02-21 Apache-2.0 releases. Their default features are disabled so the float helpers are unavailable to the exact quote path. Core's semver macros dependency could otherwise select 1.0.5, which has the later Orca License; the direct exact macros pin and lockfile test prevent that resolution. This follows the operator's license decision in the 2026-10-29 handoff 10. No later Orca source or documentation code is used.
