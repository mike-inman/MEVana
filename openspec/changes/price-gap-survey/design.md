# Design

## Context

See proposal.md. The existing Geyser capture request includes broad Whirlpool pool and tick-array filters; the survey needs an isolated request containing only exact pool public keys. Existing research PostgreSQL and Grafana assets provide the data path.

## Goals / Non-Goals

**Goals:** Verify source layouts and public fixtures, enforce live limits in the new crate, analyze prices with exact decimal math, and publish reproducible Indicative aggregates.

**Non-Goals:** Exact quotes, executable or simulated outcomes, transaction ingestion, wallet or order paths.

## Decisions

- Use a new crate and reuse only provider profile, credential parsing and transport primitives from `geyser-capture`. An isolated subscription constructor prevents broad account filters from entering the survey.
- Pin public layout sources and check real account bytes against owner/mints, then compare the decoded price against a same-pair Whirlpool within 1%. Stop if this cannot be established.
- Write bounded raw sessions outside Git, then sort confirmed account updates by slot and stable tie-breakers for deterministic analysis. Retain the latest pool price and count stale exclusions.
- Use integer fixed-point arithmetic for prices and bps calculations, with explicit truncation in the DLMM power calculation. Unknown fee propagates to NULL net gaps.
- Load aggregates in one SQL transaction and use provisioned, read-only Grafana queries with explicit time fields.

## Risks / Trade-offs

- Published lists or checked-in IDLs may be stale → verify each real account and stop on mismatch.
- Pool-account prices omit depth and execution costs → label all evidence Indicative, with no profit metric.
- Provider quota may be insufficient → measure pilot throughput and enforce total byte budget before main runs.

## Migration Plan

Add research tables and dashboard provision files; load each run atomically. Re-running a load replaces only the matching run id. Rollback is removal of survey rows and dashboard assets, leaving existing research evidence untouched.
