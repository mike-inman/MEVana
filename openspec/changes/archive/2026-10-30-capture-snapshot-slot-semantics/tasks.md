# Tasks — 2026-10-30 CDT

## Part A — offline gate

- [x] A1 Parse slot graph, confirmed ordering and parent-aware gap classifications; add synthetic skipped, fork, orphan and reconnect tests.
- [x] A2 Buffer and apply confirmed writes, drop fork writes, keep processed sensitivity, count latency in confirmed slots and bump engine version.
- [x] A3 Run handoff 09 default A/B/C and A processed; compare with handoff 11, require seven skipped slots, one fork, zero true gaps and dropped writes from slots 463132684–686.
- [x] A4 Pass pinned offline Rust, .NET, OpenSpec, hygiene, diff and non-printing credential checks before any provider use.

## Part B — one bounded capture

- [x] B1 Add opt-in Tatum snapshot client, HTTPS/credential isolation, 13-array coverage, call/byte/snapshot budgets and failure records.
- [x] B2 Add formatVersion 2 kind-2 frame records and inspection; preserve v1 behavior and tests.
- [x] B3 Replay snapshot presence/absence, context-slot precedence and `tick_array_absent` classification; test unchanged v1 reasonCode acceptance.
- [x] B4 Audit processes/sockets, run exactly one ≤600-second capture, audit shutdown and provider use.
- [x] B5 Replay new capture defaults twice and A latency, cost and processed sensitivities; verify hashes and three C# results.
- [x] B6 Complete full pinned validation, credential scans, dated closeout, OpenSpec archive, commit and push clean branch.
