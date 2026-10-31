#!/usr/bin/env python3
"""2026-10-31: Emit one repeatable transaction for Indicative survey aggregates."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path


def literal(value):
    if value is None:
        return "NULL"
    if isinstance(value, bool):
        return "TRUE" if value else "FALSE"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, (dict, list)):
        value = json.dumps(value, sort_keys=True, separators=(",", ":"))
    return "'" + str(value).replace("'", "''") + "'"


def insert(table, columns, values):
    print(f"INSERT INTO research.{table} ({','.join(columns)}) VALUES ({','.join(map(literal, values))});")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--allowlist", required=True, type=Path)
    parser.add_argument("--analyses", nargs="+", required=True, type=Path)
    parser.add_argument("--gate", required=True, type=Path)
    args = parser.parse_args()
    allowlist = json.loads(args.allowlist.read_text())
    analyses = [json.loads(path.read_text()) for path in args.analyses]
    gate = json.loads(args.gate.read_text())
    assert allowlist["evidenceKind"] == "Indicative"
    assert len({a["runId"] for a in analyses}) == len(analyses)
    run_set_id = hashlib.sha256("\n".join(sorted(a["runId"] for a in analyses)).encode()).hexdigest()
    print("BEGIN;")
    for analysis in analyses:
        assert analysis["evidenceKind"] == "Indicative"
        run_id = analysis["runId"]
        print(f"DELETE FROM research.gap_runs WHERE run_id={literal(run_id)};")
        insert("gap_runs", ["run_id", "evidence_kind", "provider", "window_start_utc", "window_end_utc", "duration_seconds", "payload_bytes", "updates", "updates_by_venue", "gaps", "stalls", "stop_reason", "allowlist_sha256", "updates_sha256"], [run_id, "Indicative", analysis["provider"], analysis["windowStartUtc"], analysis["windowEndUtc"], analysis["durationSeconds"], analysis["payloadBytes"], analysis["updates"], analysis["updatesByVenue"], analysis["gaps"], analysis["stalls"], analysis["stopReason"], analysis["allowlistSha256"], analysis["updatesSha256"]])
        for p in allowlist["pools"]:
            insert("gap_pools", ["run_id", "pool_id", "evidence_kind", "venue", "token_pair", "token_group", "mint0", "mint1", "decimals0", "decimals1", "fee_bps_micro", "base_fee_bps_micro", "fee_source", "variable_fee_modeled"], [run_id, p["address"], "Indicative", p["venue"], p["pair"], p["group"], p["mint0"], p["mint1"], p["decimals0"], p["decimals1"], p["feeBpsMicro"], p["baseFeeBpsMicro"], p["feeSource"], p["variableFeeModeled"]])
        for m in analysis["minutes"]:
            assert m["evidenceKind"] == "Indicative"
            insert("gap_minute_stats", ["run_id", "minute_utc", "token_pair", "pool_a", "pool_b", "evidence_kind", "token_group", "venue_a", "venue_b", "observation_count", "known_fee_observation_count", "median_gross_bps_micro", "maximum_gross_bps_micro", "median_net_bps_micro", "maximum_net_bps_micro", "excluded_stale_count"], [run_id, datetime.fromtimestamp(m["minuteUnix"], timezone.utc).isoformat(), m["pair"], m["poolA"], m["poolB"], "Indicative", m["group"], m["venueA"], m["venueB"], m["observationCount"], m["knownFeeObservationCount"], m["medianGrossBpsMicro"], m["maximumGrossBpsMicro"], m["medianNetBpsMicro"], m["maximumNetBpsMicro"], m["excludedStaleCount"]])
        for e in analysis["events"]:
            assert e["evidenceKind"] == "Indicative"
            t = e["startTimeUnixNs"]
            dt = None if t is None else datetime.fromtimestamp(t / 1_000_000_000, timezone.utc).isoformat()
            insert("gap_events", ["run_id", "token_pair", "pool_a", "pool_b", "threshold_bps", "start_slot", "evidence_kind", "token_group", "venue_a", "venue_b", "start_time_utc", "duration_slots", "peak_net_bps_micro", "volatility_bucket", "after_sharp_move", "open_at_end"], [run_id, e["pair"], e["poolA"], e["poolB"], e["thresholdBps"], e["startSlot"], "Indicative", e["group"], e["venueA"], e["venueB"], dt, e["durationSlots"], e["peakNetBpsMicro"], e["volatilityBucket"], e["afterSharpMove"], e["openAtEnd"]])
        for b in analysis["buckets"]:
            assert b["evidenceKind"] == "Indicative"
            insert("gap_observation_buckets", ["run_id", "token_pair", "venue_combination", "token_group", "volatility_bucket", "after_sharp_move", "evidence_kind", "observations"], [run_id, b["pair"], b["venueCombination"], b["group"], b["volatilityBucket"] or "Not measured", b["afterSharpMove"], "Indicative", b["observations"]])
    print(f"DELETE FROM research.gap_gate_results WHERE run_set_id={literal(run_set_id)};")
    for g in gate:
        assert g["evidenceKind"] == "Indicative"
        insert("gap_gate_results", ["run_set_id", "token_pair", "venue_combination", "evidence_kind", "result", "runs", "events_per_hour_10bps_micro", "median_duration_slots", "stale_exclusion_percent_micro", "all_fees_known"], [run_set_id, g["pair"], g["venueCombination"], "Indicative", g["result"], g["runs"], g["eventsPerHour10bpsMicro"], g["medianDurationSlots"], g["staleExclusionPercentMicro"], g["allFeesKnown"]])
    print("COMMIT;")


if __name__ == "__main__":
    main()
