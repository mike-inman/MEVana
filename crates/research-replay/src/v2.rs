//! Version-two route universe and multi-leg exact-in quotation.
use crate::{quote_leg, MarketState, Pair, QuoteFailure, USDC};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{collections::BTreeMap, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteV2 {
    pub shape: String,
    pub pools: Vec<Pubkey>,
    pub tokens: Vec<Pubkey>,
}

impl RouteV2 {
    pub fn fixture(&self) -> RouteFixture {
        RouteFixture {
            shape: self.shape.clone(),
            pools: self.pools.iter().map(ToString::to_string).collect(),
            tokens: self.tokens.iter().map(ToString::to_string).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteFixture {
    pub shape: String,
    pub pools: Vec<String>,
    pub tokens: Vec<String>,
}

/// Mirrors the v2 request enumerator: shape, then base58 pool sequence, then token path.
pub fn enumerate(universe: &BTreeMap<String, Vec<Pubkey>>) -> Result<Vec<RouteV2>> {
    let quote = Pubkey::from_str(USDC)?;
    let mut pool_tokens = BTreeMap::new();
    for pair in [Pair::A, Pair::B, Pair::C] {
        let (a, b) = pair.tokens();
        let tokens = [Pubkey::from_str(a)?, Pubkey::from_str(b)?];
        let ids = &universe[pair.label()];
        ensure!(
            ids.len() == pair.expected_pools(),
            "wrong v2 standard-fee universe"
        );
        for &id in ids {
            ensure!(pool_tokens.insert(id, tokens).is_none(), "duplicate pool");
        }
    }
    let mut ids: Vec<_> = pool_tokens.keys().copied().collect();
    ids.sort_by_key(ToString::to_string);
    let mut routes = Vec::new();
    for &first in &ids {
        let first_tokens = pool_tokens[&first];
        if !first_tokens.contains(&quote) {
            continue;
        }
        let middle1 = *first_tokens.iter().find(|&&t| t != quote).unwrap();
        for &second in &ids {
            if first == second {
                continue;
            }
            let second_tokens = pool_tokens[&second];
            if second_tokens.contains(&quote) && second_tokens.contains(&middle1) {
                routes.push(RouteV2 {
                    shape: "two_leg".into(),
                    pools: vec![first, second],
                    tokens: vec![quote, middle1, quote],
                });
            }
            if !second_tokens.contains(&middle1) {
                continue;
            }
            let middle2 = *second_tokens.iter().find(|&&t| t != middle1).unwrap();
            if middle2 == quote {
                continue;
            }
            for &third in &ids {
                if third == first || third == second {
                    continue;
                }
                let third_tokens = pool_tokens[&third];
                if third_tokens.contains(&middle2) && third_tokens.contains(&quote) {
                    routes.push(RouteV2 {
                        shape: "triangle".into(),
                        pools: vec![first, second, third],
                        tokens: vec![quote, middle1, middle2, quote],
                    });
                }
            }
        }
    }
    routes.sort_by(|a, b| {
        a.shape
            .cmp(&b.shape)
            .then_with(|| {
                a.pools
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .cmp(&b.pools.iter().map(ToString::to_string).collect::<Vec<_>>())
            })
            .then_with(|| {
                a.tokens
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .cmp(&b.tokens.iter().map(ToString::to_string).collect::<Vec<_>>())
            })
    });
    ensure!(routes.len() == 152, "wrong v2 route count");
    Ok(routes)
}

pub fn chain<F>(route: &RouteV2, input: u64, mut quote: F) -> std::result::Result<u64, QuoteFailure>
where
    F: FnMut(&Pubkey, Pubkey, Pubkey, u64) -> std::result::Result<u64, QuoteFailure>,
{
    let mut amount = input;
    for (i, pool) in route.pools.iter().enumerate() {
        amount = quote(pool, route.tokens[i], route.tokens[i + 1], amount)?;
    }
    Ok(amount)
}

pub fn quote_route(
    state: &MarketState,
    program: &Pubkey,
    route: &RouteV2,
    input: u64,
) -> std::result::Result<u64, QuoteFailure> {
    chain(route, input, |pool, from, to, amount| {
        quote_leg(state, program, pool, from, to, amount)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SOL, USDT};
    #[test]
    fn ordered_routes_match_dotnet_fixture() {
        let fixture: Vec<RouteFixture> = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/2026-10-30-r2-ordered-routes.json"
        ))
        .unwrap();
        let a = [
            "21gTfxAnhUDjJGZJDkTXctGFKT8TeiXx6pN1CEg9K1uW",
            "7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm",
            "83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d",
            "BSddxwYW73as8852ZTHRH13pbZEmZ96NBjayc5mSVtkZ",
            "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE",
            "FpCMFDFGYotvufJ7HrFHsWEiiQCGbkLCtwHiDnh7o28Q",
        ];
        let b = [
            "AhhoxZDmsg2snm85vPjqzYzEYESoKfb4KmTj4HrBBNwY",
            "B6LL9aCWVuo1tTcJoYvCTDqYrq1vjMfci8uHxsm4UxTR",
            "BmXVhzBHTawt5stWKBtFdjM8n33G5RoUDECE5FrRvayq",
            "FwewVm8u6tFPGewAyHmWAqad9hmF7mvqxK4mJ7iNqqGC",
            "HcoJqG325TTifs6jyWvRJ9ET4pDu12Xrt2EQKZGFmuKX",
        ];
        let c = [
            "4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4",
            "ELqXddRqcnRJXQnqZdPwBXEEBpQ6SEbCQrBe12wpHBU",
        ];
        let universe = BTreeMap::from([
            (
                "A".into(),
                a.iter().map(|id| Pubkey::from_str(id).unwrap()).collect(),
            ),
            (
                "B".into(),
                b.iter().map(|id| Pubkey::from_str(id).unwrap()).collect(),
            ),
            (
                "C".into(),
                c.iter().map(|id| Pubkey::from_str(id).unwrap()).collect(),
            ),
        ]);
        let actual = enumerate(&universe).unwrap();
        assert_eq!(actual.iter().filter(|r| r.shape == "triangle").count(), 120);
        assert_eq!(actual.iter().filter(|r| r.shape == "two_leg").count(), 32);
        assert_eq!(
            actual.iter().map(RouteV2::fixture).collect::<Vec<_>>(),
            fixture
        );
        assert_eq!(fixture[0].tokens[0], USDC);
        assert!(fixture.iter().any(|r| r.tokens.contains(&SOL.to_string())));
        assert!(fixture.iter().any(|r| r.tokens.contains(&USDT.to_string())));
    }
    #[test]
    fn chain_amounts_and_first_failure_position() {
        let ids: Vec<_> = (1..=3).map(|i| Pubkey::new_from_array([i; 32])).collect();
        let route = RouteV2 {
            shape: "triangle".into(),
            pools: ids.clone(),
            tokens: vec![Pubkey::new_unique(); 4],
        };
        let mut seen = Vec::new();
        assert_eq!(
            chain(&route, 10, |pool, _, _, amount| {
                seen.push((*pool, amount));
                Ok(amount + 1)
            }),
            Ok(13)
        );
        assert_eq!(
            seen.iter().map(|x| x.1).collect::<Vec<_>>(),
            vec![10, 11, 12]
        );
        for fail in 0..3 {
            let mut attempts = 0;
            let value = chain(&route, 10, |_, _, _, amount| {
                let i = attempts;
                attempts += 1;
                if i == fail {
                    Err(QuoteFailure::InsufficientState)
                } else {
                    Ok(amount + 1)
                }
            });
            assert_eq!(value, Err(QuoteFailure::InsufficientState));
            assert_eq!(attempts, fail + 1);
        }
    }
}
