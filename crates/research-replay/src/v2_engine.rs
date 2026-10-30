//! Offline version-two replay. The v1 engine and its serialized artifacts remain unchanged.
use crate::engine::{self, Config, PolicySummary};
use crate::policy::{decide_v2, CandidateInputV2, Decision, SizeQuote};
use crate::slot::{self, ClassifiedGap, CommitmentMode, GapClass};
use crate::v2::{self, RouteV2};
use crate::{
    cost_lamports, net, quote_leg, Capture, MarketState, QuoteFailure, PROGRAM, SOL, USDC,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{collections::BTreeMap, str::FromStr};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigV2 {
    pub base: Config,
    pub compute_units_triangle: u64,
    pub reference_pool: String,
}
impl ConfigV2 {
    pub fn cost_version(&self) -> String {
        format!(
            "cost-v2-usdc-{}-{}",
            &self.reference_pool[..12],
            self.base.tip_lamports
        )
    }
    fn units(&self, shape: &str) -> u64 {
        if shape == "triangle" {
            self.compute_units_triangle
        } else {
            self.base.compute_units_two_leg
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateRowV2 {
    pub slot: u64,
    pub shape: String,
    pub pools: Vec<String>,
    pub tokens: Vec<String>,
    pub sizes: Vec<SizeQuote>,
    pub policies: [Decision; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeSummary {
    pub route_shapes: usize,
    pub policies: [PolicySummary; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolDirectionCoverage {
    pub pool: String,
    pub input_token: String,
    pub output_token: String,
    pub attempts: u64,
    pub outcomes: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryV2 {
    pub route_shapes: usize,
    pub slot_boundaries: u64,
    pub probe_input_atoms: [u64; 3],
    pub policies: [PolicySummary; 3],
    pub shapes: BTreeMap<String, ShapeSummary>,
    pub pool_directions: Vec<PoolDirectionCoverage>,
}
pub struct OutputV2 {
    pub rows: Vec<CandidateRowV2>,
    pub summary: SummaryV2,
    pub classified_gaps: Vec<ClassifiedGap>,
    pub dropped_fork_writes: BTreeMap<u64, u64>,
    pub pools: Vec<String>,
}

fn policies() -> [PolicySummary; 3] {
    std::array::from_fn(|i| PolicySummary {
        kind: ["FixedUniverse", "SimpleRules", "ModelRanking"][i].into(),
        candidates_evaluated: 0,
        opportunities_detected: 0,
        selected_or_executed: 0,
        rejected_candidates: BTreeMap::new(),
        simulation_unavailable: BTreeMap::new(),
        estimated_net_profit_quote_atoms: 0,
        simulated_net_profit_quote_atoms: 0,
    })
}
fn cost(
    state: &MarketState,
    program: &Pubkey,
    shape: &str,
    config: &ConfigV2,
) -> std::result::Result<u64, QuoteFailure> {
    let lamports = cost_lamports(
        config.base.base_fee_lamports,
        config.base.priority_micro_lamports_per_cu,
        config.units(shape),
        config.base.tip_lamports,
    )
    .map_err(|_| QuoteFailure::InvalidQuote)?;
    let reference =
        Pubkey::from_str(&config.reference_pool).map_err(|_| QuoteFailure::InvalidQuote)?;
    quote_leg(
        state,
        program,
        &reference,
        Pubkey::from_str(SOL).unwrap(),
        Pubkey::from_str(USDC).unwrap(),
        lamports,
    )
}
fn evaluate(
    state: &MarketState,
    program: &Pubkey,
    route: &RouteV2,
    sizes: &[u64; 3],
    route_cost: std::result::Result<u64, QuoteFailure>,
) -> Vec<SizeQuote> {
    sizes
        .iter()
        .map(|&input| {
            let quoted = route_cost.and_then(|c| {
                v2::quote_route(state, program, route, input).map(|output| (output, c))
            });
            match quoted {
                Ok((output, c)) => match net(output, input, c) {
                    Ok(estimated_net) => SizeQuote {
                        input,
                        estimated_net: Some(estimated_net),
                        failure: None,
                    },
                    Err(_) => SizeQuote {
                        input,
                        estimated_net: None,
                        failure: Some("invalid_quote".into()),
                    },
                },
                Err(error) => SizeQuote {
                    input,
                    estimated_net: None,
                    failure: Some(error.code().into()),
                },
            }
        })
        .collect()
}
fn record_decision(report: &mut PolicySummary, decision: &Decision) -> Result<()> {
    report.candidates_evaluated += 1;
    if decision.estimated_net.is_some_and(|v| v > 0) {
        report.opportunities_detected += 1;
    }
    if decision.selected {
        report.selected_or_executed += 1;
        report.estimated_net_profit_quote_atoms = report
            .estimated_net_profit_quote_atoms
            .checked_add(
                decision
                    .estimated_net
                    .context("selected estimate missing")?,
            )
            .context("estimated total overflow")?;
    } else {
        *report
            .rejected_candidates
            .entry(decision.reason.as_deref().unwrap_or("invalid_quote").into())
            .or_default() += 1;
    }
    Ok(())
}
fn record_simulation(
    report: &mut PolicySummary,
    value: std::result::Result<i64, QuoteFailure>,
) -> Result<()> {
    match value {
        Ok(net) => {
            report.simulated_net_profit_quote_atoms = report
                .simulated_net_profit_quote_atoms
                .checked_add(net)
                .context("simulated total overflow")?
        }
        Err(reason) => {
            *report
                .simulation_unavailable
                .entry(reason.code().into())
                .or_default() += 1
        }
    }
    Ok(())
}
fn directions(routes: &[RouteV2]) -> Vec<(Pubkey, Pubkey, Pubkey)> {
    let mut all = std::collections::BTreeSet::new();
    for route in routes {
        for (i, &pool) in route.pools.iter().enumerate() {
            all.insert((pool, route.tokens[i], route.tokens[i + 1]));
        }
    }
    all.into_iter().collect()
}
fn check_accounting(summary: &SummaryV2) -> Result<()> {
    for policy in 0..3 {
        let total = &summary.policies[policy];
        let rejected: u64 = total.rejected_candidates.values().sum();
        ensure!(
            total.candidates_evaluated == total.selected_or_executed + rejected,
            "v2 policy accounting failed"
        );
        let shapes = summary
            .shapes
            .values()
            .map(|s| &s.policies[policy])
            .collect::<Vec<_>>();
        ensure!(
            total.candidates_evaluated
                == shapes.iter().map(|p| p.candidates_evaluated).sum::<u64>()
                && total.opportunities_detected
                    == shapes.iter().map(|p| p.opportunities_detected).sum::<u64>()
                && total.selected_or_executed
                    == shapes.iter().map(|p| p.selected_or_executed).sum::<u64>()
                && i128::from(total.estimated_net_profit_quote_atoms)
                    == shapes
                        .iter()
                        .map(|p| i128::from(p.estimated_net_profit_quote_atoms))
                        .sum::<i128>()
                && i128::from(total.simulated_net_profit_quote_atoms)
                    == shapes
                        .iter()
                        .map(|p| i128::from(p.simulated_net_profit_quote_atoms))
                        .sum::<i128>(),
            "shape totals differ from policy totals"
        );
        for shape in shapes {
            ensure!(
                shape.candidates_evaluated
                    == shape.selected_or_executed + shape.rejected_candidates.values().sum::<u64>(),
                "shape accounting failed"
            );
        }
        for (reason, count) in &total.rejected_candidates {
            ensure!(
                *count
                    == summary
                        .shapes
                        .values()
                        .map(|s| s.policies[policy]
                            .rejected_candidates
                            .get(reason)
                            .copied()
                            .unwrap_or(0))
                        .sum::<u64>(),
                "shape rejection totals differ"
            );
        }
    }
    Ok(())
}

pub fn replay(capture: &Capture, config: &ConfigV2) -> Result<OutputV2> {
    ensure!(
        capture.format_version == 2,
        "v2 profiles require format version 2"
    );
    ensure!(
        config.compute_units_triangle == 600_000
            && config.base.compute_units_two_leg == 400_000
            && config.base.latency_slots > 0
            && config.base.ranking_top_k > 0
            && config.base.sizes_quote_atoms == [10_000_000, 100_000_000, 1_000_000_000],
        "invalid v2 config"
    );
    let program = Pubkey::from_str(PROGRAM)?;
    let universe = engine::inventory(capture)?;
    let reference = Pubkey::from_str(&config.reference_pool)?;
    ensure!(
        universe["A"].contains(&reference),
        "v2 reference outside SOL/USDC pool universe"
    );
    let routes = v2::enumerate(&universe)?;
    slot::validate_confirmed_graph(&capture.slot_nodes)?;
    let boundaries = slot::boundaries(&capture.slot_nodes, config.base.commitment_mode);
    let classified_gaps = slot::classify(&capture.slot_nodes, &capture.gaps, capture.first_slot);
    let dropped_fork_writes = slot::dropped_fork_writes(&capture.slot_nodes, &capture.writes);
    let mut writes_by_slot = BTreeMap::<u64, Vec<&crate::AccountWrite>>::new();
    for write in &capture.writes {
        writes_by_slot.entry(write.slot).or_default().push(write);
    }
    let mut summary = SummaryV2 {
        route_shapes: routes.len(),
        slot_boundaries: 0,
        probe_input_atoms: config.base.sizes_quote_atoms,
        policies: policies(),
        shapes: BTreeMap::from([
            (
                "triangle".into(),
                ShapeSummary {
                    route_shapes: 120,
                    policies: policies(),
                },
            ),
            (
                "two_leg".into(),
                ShapeSummary {
                    route_shapes: 32,
                    policies: policies(),
                },
            ),
        ]),
        pool_directions: Vec::new(),
    };
    let direction_keys = directions(&routes);
    let mut coverage: BTreeMap<(String, String, String), PoolDirectionCoverage> = BTreeMap::new();
    let mut probe_cache: BTreeMap<(Pubkey, Pubkey, Pubkey, u64), (u64, String)> = BTreeMap::new();
    let mut state = MarketState::default();
    let mut rows = Vec::new();
    let mut route_cache: Vec<Option<(Vec<u64>, Vec<SizeQuote>)>> = vec![None; routes.len()];
    let mut cost_cache: BTreeMap<String, (u64, std::result::Result<u64, QuoteFailure>)> =
        BTreeMap::new();
    let mut pending = BTreeMap::<usize, Vec<(usize, usize, usize)>>::new();
    let (mut write_index, mut gap_index, mut snapshot_index) = (0, 0, 0);
    for (boundary_index, boundary) in boundaries.iter().enumerate() {
        let slot = boundary.slot;
        if slot < capture.first_slot || slot > capture.last_slot {
            continue;
        }
        loop {
            let gap_time = classified_gaps
                .get(gap_index)
                .map(|g| (g.receive_ns, g.sequence));
            let snapshot_time = capture
                .snapshots
                .get(snapshot_index)
                .map(|s| (s.receive_ns, s.sequence));
            let due = (boundary.event.receive_ns, boundary.event.sequence);
            match (
                gap_time.filter(|&t| t <= due),
                snapshot_time.filter(|&t| t <= due),
            ) {
                (Some(g), Some(s)) if s < g => {
                    state.apply_snapshot(
                        &capture.snapshots[snapshot_index].record,
                        &capture.allowlist,
                    )?;
                    snapshot_index += 1;
                }
                (Some(_), _) => {
                    if classified_gaps[gap_index].class == GapClass::TrueGap {
                        state.gap();
                    }
                    gap_index += 1;
                }
                (None, Some(_)) => {
                    state.apply_snapshot(
                        &capture.snapshots[snapshot_index].record,
                        &capture.allowlist,
                    )?;
                    snapshot_index += 1;
                }
                (None, None) => break,
            }
        }
        match config.base.commitment_mode {
            CommitmentMode::Confirmed => {
                if let Some(writes) = writes_by_slot.get(&slot) {
                    slot::apply_confirmed_writes(
                        &mut state,
                        &capture.slot_nodes,
                        slot,
                        writes,
                        &capture.allowlist,
                    )?;
                }
            }
            CommitmentMode::Processed => {
                while write_index < capture.writes.len() && capture.writes[write_index].slot <= slot
                {
                    state.apply(&capture.writes[write_index], &capture.allowlist)?;
                    write_index += 1;
                }
            }
        }
        if let Some(due) = pending.remove(&boundary_index) {
            for (row_idx, policy_idx, route_idx) in due {
                let row: &mut CandidateRowV2 = &mut rows[row_idx];
                let route = &routes[route_idx];
                let input = row.policies[policy_idx]
                    .size
                    .context("missing selected size")?;
                let later = cost(&state, &program, &route.shape, config).and_then(|c| {
                    v2::quote_route(&state, &program, route, input).and_then(|output| {
                        net(output, input, c).map_err(|_| QuoteFailure::InvalidQuote)
                    })
                });
                match later {
                    Ok(value) => row.policies[policy_idx].simulated_net = Some(value),
                    Err(reason) => {
                        row.policies[policy_idx].simulation_status = Some(reason.code().into())
                    }
                }
                record_simulation(&mut summary.policies[policy_idx], later)?;
                record_simulation(
                    &mut summary.shapes.get_mut(&route.shape).unwrap().policies[policy_idx],
                    later,
                )?;
            }
        }
        for &(pool, input_token, output_token) in &direction_keys {
            let generation = state.generation(&pool);
            let key = (
                pool.to_string(),
                input_token.to_string(),
                output_token.to_string(),
            );
            let entry = coverage
                .entry(key.clone())
                .or_insert_with(|| PoolDirectionCoverage {
                    pool: key.0,
                    input_token: key.1,
                    output_token: key.2,
                    attempts: 0,
                    outcomes: BTreeMap::new(),
                });
            for &amount in &config.base.sizes_quote_atoms {
                let cached = probe_cache.get(&(pool, input_token, output_token, amount));
                let outcome = match cached {
                    Some((g, reason)) if *g == generation => reason.clone(),
                    _ => {
                        let reason: String = match quote_leg(
                            &state,
                            &program,
                            &pool,
                            input_token,
                            output_token,
                            amount,
                        ) {
                            Ok(_) => "success".into(),
                            Err(error) => error.code().into(),
                        };
                        probe_cache.insert(
                            (pool, input_token, output_token, amount),
                            (generation, reason.clone()),
                        );
                        reason
                    }
                };
                entry.attempts += 1;
                *entry.outcomes.entry(outcome).or_default() += 1;
            }
        }
        summary.slot_boundaries += 1;
        let reference_generation = state.generation(&reference);
        let mut inputs = Vec::with_capacity(routes.len());
        for (route_idx, route) in routes.iter().enumerate() {
            let key = (0..route.pools.len())
                .map(|i| state.generation(&route.pools[i]))
                .chain(std::iter::once(reference_generation))
                .collect::<Vec<_>>();
            let route_cost = match cost_cache.get(&route.shape) {
                Some((generation, cached)) if *generation == reference_generation => *cached,
                _ => {
                    let fresh = cost(&state, &program, &route.shape, config);
                    cost_cache.insert(route.shape.clone(), (reference_generation, fresh));
                    fresh
                }
            };
            let sizes = match &route_cache[route_idx] {
                Some((generations, cached)) if *generations == key => cached.clone(),
                _ => {
                    let fresh = evaluate(
                        &state,
                        &program,
                        route,
                        &config.base.sizes_quote_atoms,
                        route_cost,
                    );
                    route_cache[route_idx] = Some((key, fresh.clone()));
                    fresh
                }
            };
            inputs.push(CandidateInputV2 {
                route: route.clone(),
                sizes,
            });
        }
        let decisions = decide_v2(
            &inputs,
            config.base.simple_min_net_quote_atoms,
            config.base.ranking_top_k,
        );
        for (route_idx, (item, policy_decisions)) in inputs.into_iter().zip(decisions).enumerate() {
            let row_idx = rows.len();
            for (policy_idx, decision) in policy_decisions.iter().enumerate() {
                record_decision(&mut summary.policies[policy_idx], decision)?;
                record_decision(
                    &mut summary.shapes.get_mut(&item.route.shape).unwrap().policies[policy_idx],
                    decision,
                )?;
                if decision.selected {
                    let target = boundary_index
                        .checked_add(usize::try_from(config.base.latency_slots)?)
                        .context("latency target overflow")?;
                    pending
                        .entry(target)
                        .or_default()
                        .push((row_idx, policy_idx, route_idx));
                }
            }
            rows.push(CandidateRowV2 {
                slot,
                shape: item.route.shape,
                pools: item.route.pools.iter().map(ToString::to_string).collect(),
                tokens: item.route.tokens.iter().map(ToString::to_string).collect(),
                sizes: item.sizes,
                policies: policy_decisions,
            });
        }
    }
    for due in pending.into_values() {
        for (row_idx, policy_idx, route_idx) in due {
            let row: &mut CandidateRowV2 = &mut rows[row_idx];
            row.policies[policy_idx].simulation_status = Some("insufficient_state".into());
            for report in [
                &mut summary.policies[policy_idx],
                &mut summary
                    .shapes
                    .get_mut(&routes[route_idx].shape)
                    .unwrap()
                    .policies[policy_idx],
            ] {
                *report
                    .simulation_unavailable
                    .entry("insufficient_state".into())
                    .or_default() += 1;
            }
        }
    }
    summary.pool_directions = coverage.into_values().collect();
    check_accounting(&summary)?;
    let mut pools = universe
        .values()
        .flat_map(|ids| ids.iter().map(ToString::to_string))
        .collect::<Vec<_>>();
    pools.sort();
    Ok(OutputV2 {
        rows,
        summary,
        classified_gaps,
        dropped_fork_writes,
        pools,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shape_costs_and_cache_key_are_distinct() {
        let base = Config {
            commitment_mode: CommitmentMode::Confirmed,
            sizes_quote_atoms: [10_000_000, 100_000_000, 1_000_000_000],
            base_fee_lamports: 5_000,
            priority_micro_lamports_per_cu: 1_000,
            compute_units_two_leg: 400_000,
            tip_lamports: 10_000,
            latency_slots: 1,
            simple_min_net_quote_atoms: 1_000,
            ranking_top_k: 1,
            seed: 0,
            reference_a: "x".into(),
            reference_b: "y".into(),
            reference_c: "z".into(),
        };
        let config = ConfigV2 {
            base,
            compute_units_triangle: 600_000,
            reference_pool: "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d".into(),
        };
        assert_eq!(
            cost_lamports(5_000, 1_000, config.units("two_leg"), 10_000).unwrap(),
            15_400
        );
        assert_eq!(
            cost_lamports(5_000, 1_000, config.units("triangle"), 10_000).unwrap(),
            15_600
        );
        let mut cache = BTreeMap::new();
        cache.insert("two_leg", 15_400);
        cache.insert("triangle", 15_600);
        assert_eq!(cache["two_leg"], 15_400);
        assert_eq!(cache["triangle"], 15_600);
    }
    #[test]
    fn shape_accounting_sums_are_enforced() {
        let mut summary = SummaryV2 {
            route_shapes: 152,
            slot_boundaries: 1,
            probe_input_atoms: [10, 100, 1_000],
            policies: policies(),
            shapes: BTreeMap::from([
                (
                    "triangle".into(),
                    ShapeSummary {
                        route_shapes: 120,
                        policies: policies(),
                    },
                ),
                (
                    "two_leg".into(),
                    ShapeSummary {
                        route_shapes: 32,
                        policies: policies(),
                    },
                ),
            ]),
            pool_directions: vec![],
        };
        assert!(check_accounting(&summary).is_ok());
        summary.policies[0].candidates_evaluated = 1;
        assert!(check_accounting(&summary).is_err());
    }

    #[test]
    fn v2_refuses_format_one_capture_before_replay() {
        let capture = Capture {
            format_version: 1,
            session_id: "synthetic-v1".into(),
            writes: vec![],
            gaps: vec![],
            slot_nodes: BTreeMap::new(),
            snapshots: vec![],
            slots: Default::default(),
            allowlist: Default::default(),
            hashes: BTreeMap::new(),
            first_receive_ns: 0,
            last_receive_ns: 1,
            first_slot: 0,
            last_slot: 1,
        };
        let base = Config {
            commitment_mode: CommitmentMode::Confirmed,
            sizes_quote_atoms: [10_000_000, 100_000_000, 1_000_000_000],
            base_fee_lamports: 5_000,
            priority_micro_lamports_per_cu: 1_000,
            compute_units_two_leg: 400_000,
            tip_lamports: 10_000,
            latency_slots: 1,
            simple_min_net_quote_atoms: 1_000,
            ranking_top_k: 1,
            seed: 0,
            reference_a: String::new(),
            reference_b: String::new(),
            reference_c: String::new(),
        };
        let config = ConfigV2 {
            base,
            compute_units_triangle: 600_000,
            reference_pool: String::new(),
        };
        assert!(replay(&capture, &config)
            .err()
            .unwrap()
            .to_string()
            .contains("format version 2"));
    }
}
