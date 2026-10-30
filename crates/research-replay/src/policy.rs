//! Deterministic policy decisions over a common per-slot route set.
use crate::v2::RouteV2;
use crate::Route;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeQuote {
    pub input: u64,
    pub estimated_net: Option<i64>,
    pub failure: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub selected: bool,
    pub size: Option<u64>,
    pub estimated_net: Option<i64>,
    pub simulated_net: Option<i64>,
    pub reason: Option<String>,
    pub simulation_status: Option<String>,
}
impl Decision {
    fn reject(size: Option<u64>, estimate: Option<i64>, reason: &str) -> Self {
        Self {
            selected: false,
            size,
            estimated_net: estimate,
            simulated_net: None,
            reason: Some(reason.into()),
            simulation_status: None,
        }
    }
    fn select(size: u64, estimate: i64) -> Self {
        Self {
            selected: true,
            size: Some(size),
            estimated_net: Some(estimate),
            simulated_net: None,
            reason: None,
            simulation_status: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateRow {
    pub pair: String,
    pub slot: u64,
    pub first_pool: String,
    pub second_pool: String,
    pub sizes: Vec<SizeQuote>,
    pub policies: [Decision; 3],
}
#[derive(Clone)]
pub struct CandidateInput {
    pub route: Route,
    pub sizes: Vec<SizeQuote>,
}
#[derive(Clone)]
pub struct CandidateInputV2 {
    pub route: RouteV2,
    pub sizes: Vec<SizeQuote>,
}

fn failure(sizes: &[SizeQuote]) -> &'static str {
    for reason in [
        "state_gap",
        "insufficient_state",
        "tick_array_absent",
        "adaptive_fee_unsupported",
        "invalid_quote",
    ] {
        if sizes.iter().any(|s| s.failure.as_deref() == Some(reason)) {
            return reason;
        }
    }
    "insufficient_state"
}
fn best(sizes: &[SizeQuote]) -> Option<(u64, i64)> {
    sizes
        .iter()
        .filter_map(|s| s.estimated_net.map(|n| (s.input, n)))
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
}
fn ratio_cmp(a: (u64, i64), b: (u64, i64)) -> Ordering {
    (i128::from(a.1) * i128::from(b.0)).cmp(&(i128::from(b.1) * i128::from(a.0)))
}

/// Policy indices: FixedUniverse, SimpleRules, ModelRanking.
/// One decision bucket is assigned to each route for each policy.
pub fn decide(inputs: &[CandidateInput], min_net: i64, top_k: usize) -> Vec<[Decision; 3]> {
    let mut result = Vec::with_capacity(inputs.len());
    for item in inputs {
        let fixed = &item.sizes[0];
        let fixed_decision = match (fixed.estimated_net, fixed.failure.as_deref()) {
            (Some(net), _) if net > 0 => Decision::select(fixed.input, net),
            (Some(net), _) => Decision::reject(Some(fixed.input), Some(net), "below_threshold"),
            (_, Some(reason)) => Decision::reject(Some(fixed.input), None, reason),
            _ => Decision::reject(Some(fixed.input), None, "insufficient_state"),
        };
        let baseline = match best(&item.sizes) {
            Some((size, net)) if net >= min_net && net > 0 => Decision::select(size, net),
            Some((size, net)) => Decision::reject(Some(size), Some(net), "below_threshold"),
            None => Decision::reject(None, None, failure(&item.sizes)),
        };
        result.push([fixed_decision, baseline.clone(), baseline]);
    }
    for policy in [1, 2] {
        let mut order: Vec<usize> = (0..inputs.len())
            .filter(|&i| result[i][policy].selected)
            .collect();
        order.sort_by(|&a, &b| {
            let da = &result[a][policy];
            let db = &result[b][policy];
            let left = (da.size.unwrap(), da.estimated_net.unwrap());
            let right = (db.size.unwrap(), db.estimated_net.unwrap());
            let relative = if policy == 1 {
                left.1.cmp(&right.1)
            } else {
                ratio_cmp(left, right)
            };
            relative
                .reverse()
                .then_with(|| inputs[a].route.first.cmp(&inputs[b].route.first))
                .then_with(|| inputs[a].route.second.cmp(&inputs[b].route.second))
        });
        let mut used = std::collections::BTreeSet::new();
        let mut count = 0;
        for i in order {
            let route = &inputs[i].route;
            if used.contains(&route.first) || used.contains(&route.second) {
                result[i][policy].selected = false;
                result[i][policy].reason = Some("pool_conflict".into());
            } else if policy == 2 && count >= top_k {
                result[i][policy].selected = false;
                result[i][policy].reason = Some("not_top_k".into());
            } else {
                used.insert(route.first);
                used.insert(route.second);
                count += 1;
            }
        }
    }
    result
}

/// The v2 policies compete across all shapes and consider every pool on each route.
pub fn decide_v2(inputs: &[CandidateInputV2], min_net: i64, top_k: usize) -> Vec<[Decision; 3]> {
    let mut result = Vec::with_capacity(inputs.len());
    for item in inputs {
        let fixed = &item.sizes[0];
        let fixed_decision = match (fixed.estimated_net, fixed.failure.as_deref()) {
            (Some(net), _) if net > 0 => Decision::select(fixed.input, net),
            (Some(net), _) => Decision::reject(Some(fixed.input), Some(net), "below_threshold"),
            (_, Some(reason)) => Decision::reject(Some(fixed.input), None, reason),
            _ => Decision::reject(Some(fixed.input), None, "insufficient_state"),
        };
        let baseline = match best(&item.sizes) {
            Some((size, net)) if net >= min_net && net > 0 => Decision::select(size, net),
            Some((size, net)) => Decision::reject(Some(size), Some(net), "below_threshold"),
            None => Decision::reject(None, None, failure(&item.sizes)),
        };
        result.push([fixed_decision, baseline.clone(), baseline]);
    }
    for policy in [1, 2] {
        let mut order: Vec<usize> = (0..inputs.len())
            .filter(|&i| result[i][policy].selected)
            .collect();
        order.sort_by(|&a, &b| {
            let left = (
                result[a][policy].size.unwrap(),
                result[a][policy].estimated_net.unwrap(),
            );
            let right = (
                result[b][policy].size.unwrap(),
                result[b][policy].estimated_net.unwrap(),
            );
            let relative = if policy == 1 {
                left.1.cmp(&right.1)
            } else {
                ratio_cmp(left, right)
            };
            relative
                .reverse()
                .then_with(|| inputs[a].route.pools.cmp(&inputs[b].route.pools))
        });
        let mut used = std::collections::BTreeSet::new();
        let mut count = 0;
        for i in order {
            let route = &inputs[i].route;
            if route.pools.iter().any(|pool| used.contains(pool)) {
                result[i][policy].selected = false;
                result[i][policy].reason = Some("pool_conflict".into());
            } else if policy == 2 && count >= top_k {
                result[i][policy].selected = false;
                result[i][policy].reason = Some("not_top_k".into());
            } else {
                used.extend(route.pools.iter().copied());
                count += 1;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    fn input(first: u8, second: u8, values: &[(u64, i64)]) -> CandidateInput {
        CandidateInput {
            route: Route {
                first: Pubkey::new_from_array([first; 32]),
                second: Pubkey::new_from_array([second; 32]),
                base: Pubkey::new_unique(),
                quote: Pubkey::new_unique(),
            },
            sizes: values
                .iter()
                .map(|&(size, net)| SizeQuote {
                    input: size,
                    estimated_net: Some(net),
                    failure: None,
                })
                .collect(),
        }
    }
    #[test]
    fn synthetic_policy_buckets_and_conflicts() {
        let rows = decide(
            &[
                input(1, 2, &[(10, 2), (100, 3)]),
                input(2, 3, &[(10, 1), (100, 4)]),
                input(4, 5, &[(10, -1), (100, 5)]),
            ],
            2,
            1,
        );
        assert_eq!(rows.iter().filter(|r| r[0].selected).count(), 2);
        assert_eq!(rows.iter().filter(|r| r[1].selected).count(), 2);
        assert_eq!(rows.iter().filter(|r| r[2].selected).count(), 1);
        assert!(rows
            .iter()
            .all(|r| r.iter().all(|p| p.selected ^ p.reason.is_some())));
    }
    #[test]
    fn synthetic_ratio_ranking_uses_integer_cross_product() {
        assert_eq!(ratio_cmp((10, 2), (100, 10)), Ordering::Greater);
    }
    #[test]
    fn v2_third_pool_conflict_and_order_are_deterministic() {
        fn item(pools: &[u8], net: i64) -> CandidateInputV2 {
            CandidateInputV2 {
                route: RouteV2 {
                    shape: if pools.len() == 3 {
                        "triangle"
                    } else {
                        "two_leg"
                    }
                    .into(),
                    pools: pools
                        .iter()
                        .map(|i| Pubkey::new_from_array([*i; 32]))
                        .collect(),
                    tokens: vec![Pubkey::new_unique(); pools.len() + 1],
                },
                sizes: vec![SizeQuote {
                    input: 10,
                    estimated_net: Some(net),
                    failure: None,
                }],
            }
        }
        let inputs = [item(&[1, 2, 3], 10), item(&[4, 3], 9), item(&[5, 6], 8)];
        let first = decide_v2(&inputs, 1, 1);
        let second = decide_v2(&inputs, 1, 1);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first[1][1].reason.as_deref(), Some("pool_conflict"));
        assert_eq!(first[1][2].reason.as_deref(), Some("pool_conflict"));
        assert_eq!(first[2][2].reason.as_deref(), Some("not_top_k"));
        assert!(first[0][0].selected && first[1][0].selected);
    }
}
