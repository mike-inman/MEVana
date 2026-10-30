How to integrate

2026-10-29 research note: `exact::quote_exact_in` and `exact::quote_exact_out` are the integer-only standard-fee quoting APIs for research. Pass a captured pool and keyed fixed/dynamic tick arrays. Adaptive-fee pools and missing arrays return typed errors. Orca core/macros are pinned to the Apache-2.0 1.0.4 releases; see `THIRD_PARTY.md`. The 1.0.4 swap result exposes amounts and fees, not next pool state.

`math.rs`, `quoter.rs`, and `tick_window.rs` are legacy approximations still used by `edge-assembler`. Do not use their output for research results.


Add this crate to your workspace and build: cargo build -p whirlpool-core

In your Orca “edge-assembler” microservice:

Use decode::decode_whirlpool on pool account updates.

Derive the three tick-array PDAs from pda::three_tick_arrays_around and pda::derive_tick_array_address.

Seed them once via RPC (getMultipleAccounts) using the example code, decode with decode::decode_tick_array, and cache them in your per-pool state.

Build a TickWindow from your cached arrays and call tick_window::find_nearest_initialized_ticks to get the immediate boundaries.

Optionally use quoter::ClmmQuoter::size_to_next_tick_in(Direction) to emit size-to-next-tick hints in your EdgeDelta.

Keep listening to Geyser for tick-array updates; when a tick-array changes, refresh the cached TickArray for that pool and recompute hints.



Notes


The quoter here uses exact 256-bit integer math within a single tick, and a conservative linear fraction to estimate output when you don’t fully consume to the boundary. That’s enough for prefiltering and for your first signals. Next, we can replace it with an exact piecewise solver that walks multiple ticks and recomputes sqrt price per step (still integer).

sqrt_price at a tick is currently approximated with f64 for convenience; we’ll replace with integer exponentiation (or a precomputed table) in the next iteration.

All public functions return integers; keep floats only for logs.


Add an exact multi-tick quote_out implementation (integer Q64.64) to quoter.rs.

Add a small EdgeDelta struct and serde/bincode encoding you can push to Kafka.

Provide a Raydium CPMM module next (reserves decode + quoter) so you can do your first cross-venue arb.
