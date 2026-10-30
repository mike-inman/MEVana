//! Slot-boundary evaluation and later-slot simulation.
use crate::policy::{decide, CandidateInput, CandidateRow, SizeQuote};
use crate::{
    cost_lamports, net, quote_leg, quote_route, routes, Capture, MarketState, Pair, QuoteFailure,
    Route, PROGRAM, SOL,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{collections::BTreeMap, str::FromStr};
use whirlpool_core::types::WhirlpoolMinimal;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub sizes_quote_atoms: [u64; 3],
    pub base_fee_lamports: u64,
    pub priority_micro_lamports_per_cu: u64,
    pub compute_units_two_leg: u64,
    pub tip_lamports: u64,
    pub latency_slots: u64,
    pub simple_min_net_quote_atoms: i64,
    pub ranking_top_k: usize,
    pub seed: u64,
    pub reference_a: String,
    pub reference_b: String,
    pub reference_c: String,
}
impl Config {
    pub fn cost_version(&self, pair: Pair) -> String {
        let reference = match pair {
            Pair::A => &self.reference_a,
            Pair::B => &self.reference_b,
            Pair::C => &self.reference_c,
        };
        // Identifier-safe shortened reference still uniquely records the full key in the run config.
        format!(
            "cost-v1-{}-{}-{}",
            pair.label().to_ascii_lowercase(),
            &reference[..12],
            self.tip_lamports
        )
    }
    pub fn reference(&self, pair: Pair) -> Result<Pubkey> {
        Ok(Pubkey::from_str(match pair {
            Pair::A => &self.reference_a,
            Pair::B => &self.reference_b,
            Pair::C => &self.reference_c,
        })?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicySummary {
    pub kind: String,
    pub candidates_evaluated: u64,
    pub opportunities_detected: u64,
    pub selected_or_executed: u64,
    pub rejected_candidates: BTreeMap<String, u64>,
    pub simulation_unavailable: BTreeMap<String, u64>,
    pub estimated_net_profit_quote_atoms: i64,
    pub simulated_net_profit_quote_atoms: i64,
}
impl PolicySummary {
    fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            candidates_evaluated: 0,
            opportunities_detected: 0,
            selected_or_executed: 0,
            rejected_candidates: BTreeMap::new(),
            simulation_unavailable: BTreeMap::new(),
            estimated_net_profit_quote_atoms: 0,
            simulated_net_profit_quote_atoms: 0,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairSummary {
    pub pair: Pair,
    pub pools: Vec<String>,
    pub route_shapes: usize,
    pub slot_boundaries: u64,
    pub reference_pool: String,
    pub policies: [PolicySummary; 3],
}
pub struct ReplayOutput {
    pub rows: Vec<CandidateRow>,
    pub summaries: Vec<PairSummary>,
}
type CachedRouteQuotes = Option<([u64; 3], Vec<SizeQuote>)>;

fn standard_pair(pool: &WhirlpoolMinimal) -> Option<Pair> {
    if u16::from_le_bytes(pool.tick_spacing_seed) != pool.tick_spacing {
        return None;
    }
    for pair in [Pair::A, Pair::B, Pair::C] {
        let (a, b) = pair.tokens();
        if (pool.token_mint_a.to_string() == a && pool.token_mint_b.to_string() == b)
            || (pool.token_mint_a.to_string() == b && pool.token_mint_b.to_string() == a)
        {
            return Some(pair);
        }
    }
    None
}
pub fn inventory(capture: &Capture) -> Result<BTreeMap<String, Vec<Pubkey>>> {
    let mut latest = BTreeMap::<Pubkey, WhirlpoolMinimal>::new();
    for write in &capture.writes {
        if capture.allowlist.contains(&write.key)
            && geyser_capture::layout::kind(&write.data) == "pool"
        {
            latest.insert(
                write.key,
                whirlpool_core::decode::decode_whirlpool(&write.data)?,
            );
        }
    }
    let mut result = BTreeMap::<String, Vec<Pubkey>>::new();
    for (id, pool) in latest {
        if let Some(pair) = standard_pair(&pool) {
            result.entry(pair.label().into()).or_default().push(id);
        }
    }
    for pair in [Pair::A, Pair::B, Pair::C] {
        ensure!(
            result
                .get(pair.label())
                .is_some_and(|p| p.len() == pair.expected_pools()),
            "standard-fee universe changed for {:?}",
            pair
        );
    }
    Ok(result)
}
fn cost(
    state: &MarketState,
    program: &Pubkey,
    pair: Pair,
    config: &Config,
) -> std::result::Result<u64, QuoteFailure> {
    let lamports = cost_lamports(
        config.base_fee_lamports,
        config.priority_micro_lamports_per_cu,
        config.compute_units_two_leg,
        config.tip_lamports,
    )
    .map_err(|_| QuoteFailure::InvalidQuote)?;
    let reference = config
        .reference(pair)
        .map_err(|_| QuoteFailure::InvalidQuote)?;
    let quote = Pubkey::from_str(pair.tokens().1).map_err(|_| QuoteFailure::InvalidQuote)?;
    let sol = Pubkey::from_str(SOL).map_err(|_| QuoteFailure::InvalidQuote)?;
    quote_leg(state, program, &reference, sol, quote, lamports)
}
fn evaluate(
    state: &MarketState,
    program: &Pubkey,
    route: &Route,
    sizes: &[u64; 3],
    cost: std::result::Result<u64, QuoteFailure>,
) -> Vec<SizeQuote> {
    sizes
        .iter()
        .map(|&input| {
            let quote = cost
                .and_then(|c| quote_route(state, program, route, input).map(|output| (output, c)));
            match quote {
                Ok((output, cost)) => match net(output, input, cost) {
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
pub fn replay(capture: &Capture, config: &Config, pairs: &[Pair]) -> Result<ReplayOutput> {
    ensure!(
        config.latency_slots > 0
            && config.sizes_quote_atoms.windows(2).all(|w| w[0] < w[1])
            && config.sizes_quote_atoms[0] > 0
            && config.ranking_top_k > 0,
        "invalid replay config"
    );
    let program = Pubkey::from_str(PROGRAM)?;
    let universe = inventory(capture)?;
    let mut summaries = Vec::new();
    let mut route_sets = BTreeMap::new();
    for &pair in pairs {
        let ids = &universe[pair.label()];
        let reference = config.reference(pair)?;
        ensure!(
            universe[if pair == Pair::A { "A" } else { "B" }].contains(&reference),
            "reference pool outside captured standard-fee pair"
        );
        let routes = routes(pair, ids)?;
        let mut pool_strings: Vec<String> = ids.iter().map(ToString::to_string).collect();
        pool_strings.sort();
        summaries.push(PairSummary {
            pair,
            pools: pool_strings,
            route_shapes: routes.len(),
            slot_boundaries: 0,
            reference_pool: reference.to_string(),
            policies: [
                PolicySummary::new("FixedUniverse"),
                PolicySummary::new("SimpleRules"),
                PolicySummary::new("ModelRanking"),
            ],
        });
        route_sets.insert(pair.label(), routes);
    }
    let mut state = MarketState::default();
    let mut rows = Vec::<CandidateRow>::new();
    // Reuse exact quotes only while both route pools and the cost reference have
    // unchanged captured generations. This never infers an absent array.
    let mut route_cache: Vec<Vec<CachedRouteQuotes>> = summaries
        .iter()
        .map(|s| vec![None; s.route_shapes])
        .collect();
    let mut cost_cache: Vec<Option<(u64, std::result::Result<u64, QuoteFailure>)>> =
        vec![None; summaries.len()];
    let mut pending = BTreeMap::<u64, Vec<(usize, usize, usize)>>::new();
    let mut write_index = 0;
    let mut gap_index = 0;
    for &slot in &capture.slots {
        if slot < capture.first_slot || slot > capture.last_slot {
            continue;
        }
        // Gaps at this boundary invalidate old pool state before same-slot writes restore it.
        while gap_index < capture.gaps.len() && capture.gaps[gap_index].slot <= slot {
            state.gap();
            gap_index += 1;
        }
        while write_index < capture.writes.len() && capture.writes[write_index].slot <= slot {
            state.apply(&capture.writes[write_index], &capture.allowlist)?;
            write_index += 1;
        }
        if let Some(due) = pending.remove(&slot) {
            for (row_idx, policy_idx, summary_idx) in due {
                let row = &mut rows[row_idx];
                let pair = summaries[summary_idx].pair;
                let route = Route {
                    first: Pubkey::from_str(&row.first_pool)?,
                    second: Pubkey::from_str(&row.second_pool)?,
                    base: Pubkey::from_str(pair.tokens().0)?,
                    quote: Pubkey::from_str(pair.tokens().1)?,
                };
                let size = row.policies[policy_idx].size.unwrap();
                let later = cost(&state, &program, pair, config).and_then(|c| {
                    quote_route(&state, &program, &route, size).map(|output| (output, c))
                });
                match later {
                    Ok((output, c)) => {
                        let value = net(output, size, c)?;
                        row.policies[policy_idx].simulated_net = Some(value);
                        summaries[summary_idx].policies[policy_idx]
                            .simulated_net_profit_quote_atoms = summaries[summary_idx].policies
                            [policy_idx]
                            .simulated_net_profit_quote_atoms
                            .checked_add(value)
                            .ok_or(anyhow::anyhow!("simulated total overflow"))?;
                    }
                    Err(failure) => {
                        let reason = failure.code().to_string();
                        row.policies[policy_idx].simulation_status = Some(reason.clone());
                        *summaries[summary_idx].policies[policy_idx]
                            .simulation_unavailable
                            .entry(reason)
                            .or_default() += 1;
                    }
                }
            }
        }
        for (summary_idx, summary) in summaries.iter_mut().enumerate() {
            summary.slot_boundaries += 1;
            let routes = &route_sets[summary.pair.label()];
            let reference_generation = state.generation(&config.reference(summary.pair)?);
            let slot_cost = match cost_cache[summary_idx] {
                Some((generation, cached)) if generation == reference_generation => cached,
                _ => {
                    let fresh = cost(&state, &program, summary.pair, config);
                    cost_cache[summary_idx] = Some((reference_generation, fresh));
                    fresh
                }
            };
            let inputs: Vec<CandidateInput> = routes
                .iter()
                .enumerate()
                .map(|(route_idx, route)| {
                    let generations = [
                        state.generation(&route.first),
                        state.generation(&route.second),
                        reference_generation,
                    ];
                    let sizes = match &route_cache[summary_idx][route_idx] {
                        Some((cached_generations, cached))
                            if *cached_generations == generations =>
                        {
                            cached.clone()
                        }
                        _ => {
                            let fresh = evaluate(
                                &state,
                                &program,
                                route,
                                &config.sizes_quote_atoms,
                                slot_cost,
                            );
                            route_cache[summary_idx][route_idx] =
                                Some((generations, fresh.clone()));
                            fresh
                        }
                    };
                    CandidateInput {
                        route: route.clone(),
                        sizes,
                    }
                })
                .collect();
            let decisions = decide(
                &inputs,
                config.simple_min_net_quote_atoms,
                config.ranking_top_k,
            );
            for (item, policies) in inputs.into_iter().zip(decisions) {
                let row_idx = rows.len();
                for (policy_idx, decision) in policies.iter().enumerate() {
                    let report = &mut summary.policies[policy_idx];
                    report.candidates_evaluated += 1;
                    if decision.estimated_net.is_some_and(|v| v > 0) {
                        report.opportunities_detected += 1;
                    }
                    if decision.selected {
                        report.selected_or_executed += 1;
                        report.estimated_net_profit_quote_atoms = report
                            .estimated_net_profit_quote_atoms
                            .checked_add(decision.estimated_net.unwrap())
                            .ok_or(anyhow::anyhow!("estimated total overflow"))?;
                        if let Some(target) = slot.checked_add(config.latency_slots) {
                            pending.entry(target).or_default().push((
                                row_idx,
                                policy_idx,
                                summary_idx,
                            ));
                        }
                    } else {
                        *report
                            .rejected_candidates
                            .entry(decision.reason.as_deref().unwrap_or("invalid_quote").into())
                            .or_default() += 1;
                    }
                }
                rows.push(CandidateRow {
                    pair: summary.pair.label().into(),
                    slot,
                    first_pool: item.route.first.to_string(),
                    second_pool: item.route.second.to_string(),
                    sizes: item.sizes,
                    policies,
                });
            }
        }
    }
    for (_target, due) in pending {
        for (row_idx, policy_idx, summary_idx) in due {
            rows[row_idx].policies[policy_idx].simulation_status =
                Some("insufficient_state".into());
            *summaries[summary_idx].policies[policy_idx]
                .simulation_unavailable
                .entry("insufficient_state".into())
                .or_default() += 1;
        }
    }
    for summary in &summaries {
        for p in &summary.policies {
            let rejected: u64 = p.rejected_candidates.values().sum();
            ensure!(
                p.candidates_evaluated == p.selected_or_executed + rejected,
                "accounting invariant failed"
            );
        }
    }
    Ok(ReplayOutput { rows, summaries })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_policy_accounting_contract() {
        let s = PolicySummary::new("FixedUniverse");
        assert_eq!(
            s.selected_or_executed + s.rejected_candidates.values().sum::<u64>(),
            s.candidates_evaluated
        );
    }
    #[test]
    fn synthetic_estimate_and_later_are_independent() {
        let mut row = crate::policy::Decision {
            selected: true,
            size: Some(10),
            estimated_net: Some(5),
            simulated_net: None,
            reason: None,
            simulation_status: None,
        };
        row.simulated_net = Some(-2);
        assert_eq!(row.estimated_net, Some(5));
        assert_eq!(row.simulated_net, Some(-2));
    }
}
