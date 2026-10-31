//! Fixed predeclared gate: three runs, six 10-bps events/hour, two-slot median,
//! known fees and fewer than 20% stale exclusions.
use crate::analysis::Analysis;
use anyhow::{ensure, Result};
use chrono::DateTime;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateResult {
    pub evidence_kind: String,
    pub pair: String,
    pub venue_combination: String,
    pub result: String,
    pub runs: usize,
    pub events_per_hour_10bps_micro: Option<u64>,
    pub median_duration_slots: Option<u64>,
    pub stale_exclusion_percent_micro: Option<u64>,
    pub all_fees_known: bool,
}

pub fn evaluate(analyses: &[Analysis]) -> Result<Vec<GateResult>> {
    let mut ids = BTreeSet::new();
    for a in analyses {
        ensure!(
            a.evidence_kind == "Indicative" && ids.insert(&a.run_id),
            "duplicate or invalid analysis"
        );
    }
    let mut groups =
        BTreeMap::<(String, String), Vec<(&Analysis, &crate::analysis::SegmentStat)>>::new();
    for a in analyses {
        for s in &a.segments {
            groups
                .entry((s.pair.clone(), s.venue_combination.clone()))
                .or_default()
                .push((a, s));
        }
    }
    let mut out = Vec::new();
    for ((pair, venue_combination), runs) in groups {
        let mut observations = 0u64;
        let mut stale = 0u64;
        let mut known = 0u64;
        let mut events = 0u64;
        let mut duration = Vec::new();
        let mut seconds = 0i64;
        for (a, s) in &runs {
            observations += s.observation_count;
            stale += s.stale_count;
            known += s.known_fee_count;
            events += s.events_10bps;
            duration.extend_from_slice(&s.event_duration_slots);
            let start = DateTime::parse_from_rfc3339(&a.window_start_utc)?;
            let end = DateTime::parse_from_rfc3339(&a.window_end_utc)?;
            seconds += (end - start).num_seconds().max(0);
        }
        duration.sort_unstable();
        let median = if duration.is_empty() {
            None
        } else {
            Some(duration[(duration.len() - 1) / 2])
        };
        let rate = if seconds > 0 {
            Some(events.saturating_mul(3_600_000_000) / seconds as u64)
        } else {
            None
        };
        let stale_pct = if observations > 0 {
            Some(stale.saturating_mul(100_000_000) / observations)
        } else {
            None
        };
        let all_fees_known = known == observations.saturating_sub(stale);
        let result = if runs.len() < 3 || observations == 0 || seconds == 0 {
            "too_little_data"
        } else if rate.is_some_and(|v| v >= 6_000_000)
            && median.is_some_and(|v| v >= 2)
            && all_fees_known
            && stale_pct.is_some_and(|v| v < 20_000_000)
        {
            "advance"
        } else {
            "fail"
        };
        out.push(GateResult {
            evidence_kind: "Indicative".into(),
            pair,
            venue_combination,
            result: result.into(),
            runs: runs.len(),
            events_per_hour_10bps_micro: rate,
            median_duration_slots: median,
            stale_exclusion_percent_micro: stale_pct,
            all_fees_known,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::SegmentStat;
    fn run(id: &str, events: u64, durations: Vec<u64>, stale: u64, known: u64) -> Analysis {
        Analysis {
            evidence_kind: "Indicative".into(),
            run_id: id.into(),
            provider: "mock".into(),
            window_start_utc: "2026-10-31T00:00:00+00:00".into(),
            window_end_utc: "2026-10-31T01:00:00+00:00".into(),
            duration_seconds: 3600,
            payload_bytes: 0,
            updates: 0,
            gaps: 0,
            stalls: 0,
            stop_reason: "duration".into(),
            allowlist_sha256: "test".into(),
            updates_sha256: "test".into(),
            updates_by_venue: Default::default(),
            minutes: vec![],
            events: vec![],
            segments: vec![SegmentStat {
                evidence_kind: "Indicative".into(),
                run_id: id.into(),
                pair: "SOL/USDC".into(),
                venue_combination: "Orca/Raydium".into(),
                observation_count: 100,
                stale_count: stale,
                known_fee_count: known,
                events_10bps: events,
                event_duration_slots: durations,
            }],
            buckets: vec![],
        }
    }
    #[test]
    fn fixed_thresholds_and_three_run_rule() {
        let a = run("a", 6, vec![2; 6], 19, 81);
        let b = run("b", 6, vec![2; 6], 19, 81);
        let c = run("c", 6, vec![2; 6], 19, 81);
        assert_eq!(evaluate(&[a, b]).unwrap()[0].result, "too_little_data");
        assert_eq!(
            evaluate(&[
                run("a", 6, vec![2; 6], 19, 81),
                run("b", 6, vec![2; 6], 19, 81),
                c
            ])
            .unwrap()[0]
                .result,
            "advance"
        );
        assert_eq!(
            evaluate(&[
                run("a", 6, vec![2; 6], 20, 80),
                run("b", 6, vec![2; 6], 20, 80),
                run("c", 6, vec![2; 6], 20, 80)
            ])
            .unwrap()[0]
                .result,
            "fail"
        );
    }
}
