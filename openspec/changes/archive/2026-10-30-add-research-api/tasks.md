# Tasks — 2026-10-30 CDT

## 1. Store

- [x] 1.1 Add strict catalog, request and result parsers, with each request parser refusing the other contract version.
- [x] 1.2 Add the fail-closed artifact store and committed seed; verify each check with a tampered copy and seed integrity against fixtures.

## 2. API

- [x] 2.1 Add the five read-only routes, 405 and 404 handling, and method, template and status request logging.
- [x] 2.2 Add experiment summaries, detail, per-policy coverage and labeled diagnostic probes; verify evidence, quote asset, null-versus-zero and hash recomputation.
- [x] 2.3 Add the loopback bind guard and reserve port 8098; verify rejected addresses.
- [x] 2.4 Commit golden responses and verify determinism across two starts.

## 3. Closeout

- [x] 3.1 Run the smoke test, confirm nothing remains listening, run the full validation list and write the dated closeout.

## Workflow follow-up

- Sync specs, archive the OpenSpec change and push the branch; do not merge.
