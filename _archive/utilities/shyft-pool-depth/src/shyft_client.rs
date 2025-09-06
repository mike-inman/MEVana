use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Shyft API client for fetching pool data
pub struct ShyftClient {
    client: Client,
    api_key: String,
    network: String,
}

/// Pool data from Shyft GraphQL API
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShyftPoolData {
    pub pubkey: String,
    pub token_mint_a: String,
    pub token_mint_b: String,
    pub fee_rate: i64,
    pub tick_current_index: i32,
    pub sqrt_price: String,
    pub liquidity: Option<String>,
}

/// GraphQL response structure
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct WhirlpoolRows {
    #[serde(rename = "ORCA_WHIRLPOOLS_whirlpool")]
    rows: Vec<ShyftPoolData>,
}

impl ShyftClient {
    /// Create a new Shyft API client
    pub fn new(api_key: String, network: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            network,
        }
    }

    /// Get pool details by address
    pub async fn get_pool_details(&self, pool_address: &str) -> Result<Option<ShyftPoolData>> {
        let query = format!(
            r#"
            query GetPool($address: String!) {{
                ORCA_WHIRLPOOLS_whirlpool(
                    where: {{ pubkey: {{ _eq: $address }} }}
                    limit: 1
                ) {{
                    pubkey
                    tokenMintA
                    tokenMintB
                    feeRate
                    tickCurrentIndex
                    sqrtPrice
                    liquidity
                }}
            }}
            "#,
            address = pool_address
        );

        let response: GraphQLResponse<WhirlpoolRows> = self
            .client
            .post(&self.graphql_endpoint())
            .json(&serde_json::json!({
                "query": query,
                "variables": { "address": pool_address }
            }))
            .send()
            .await
            .context("Failed to send GraphQL request")?
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        Ok(response.data.rows.into_iter().next())
    }

    /// Get multiple pools with pagination
    pub async fn get_pools(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ShyftPoolData>> {
        let query = format!(
            r#"
            query GetPools($limit: Int!, $offset: Int!) {{
                ORCA_WHIRLPOOLS_whirlpool(
                    limit: $limit
                    offset: $offset
                    order_by: {{ liquidity: desc }}
                ) {{
                    pubkey
                    tokenMintA
                    tokenMintB
                    feeRate
                    tickCurrentIndex
                    sqrtPrice
                    liquidity
                }}
            }}
            "#,
            limit = limit,
            offset = offset
        );

        let response: GraphQLResponse<WhirlpoolRows> = self
            .client
            .post(&self.graphql_endpoint())
            .json(&serde_json::json!({
                "query": query,
                "variables": { "limit": limit, "offset": offset }
            }))
            .send()
            .await
            .context("Failed to send GraphQL request")?
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        Ok(response.data.rows)
    }

    /// Get pools for a specific token
    pub async fn get_pools_for_token(&self, token_mint: &str) -> Result<Vec<ShyftPoolData>> {
        let query = format!(
            r#"
            query GetPoolsForToken($tokenMint: String!) {{
                ORCA_WHIRLPOOLS_whirlpool(
                    where: {{
                        _or: [
                            {{ tokenMintA: {{ _eq: $tokenMint }} }},
                            {{ tokenMintB: {{ _eq: $tokenMint }} }}
                        ]
                    }}
                    order_by: {{ liquidity: desc }}
                ) {{
                    pubkey
                    tokenMintA
                    tokenMintB
                    feeRate
                    tickCurrentIndex
                    sqrtPrice
                    liquidity
                }}
            }}
            "#,
            tokenMint = token_mint
        );

        let response: GraphQLResponse<WhirlpoolRows> = self
            .client
            .post(&self.graphql_endpoint())
            .json(&serde_json::json!({
                "query": query,
                "variables": { "tokenMint": token_mint }
            }))
            .send()
            .await
            .context("Failed to send GraphQL request")?
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        Ok(response.data.rows)
    }

    /// Get the GraphQL endpoint URL
    fn graphql_endpoint(&self) -> String {
        format!(
            "https://programs.shyft.to/v0/graphql/?api_key={}&network={}",
            self.api_key, self.network
        )
    }
}

/// Convert Shyft pool data to TickArray-compatible format
impl ShyftPoolData {
    /// Get current tick index
    pub fn current_tick(&self) -> i64 {
        self.tick_current_index as i64
    }

    /// Get sqrt price as u128
    pub fn sqrt_price_u128(&self) -> Result<u128> {
        self.sqrt_price
            .parse::<u128>()
            .context("Failed to parse sqrt_price as u128")
    }

    /// Get fee rate in basis points
    pub fn fee_bps(&self) -> u16 {
        (self.fee_rate / 100) as u16
    }

    /// Check if this pool has valid tick data
    pub fn has_valid_tick_data(&self) -> bool {
        self.tick_current_index != 0 && !self.sqrt_price.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_bps_conversion() {
        let pool = ShyftPoolData {
            pubkey: "test".to_string(),
            token_mint_a: "mint_a".to_string(),
            token_mint_b: "mint_b".to_string(),
            fee_rate: 20000, // 0.2%
            tick_current_index: 1000,
            sqrt_price: "1000000".to_string(),
            liquidity: Some("1000000".to_string()),
        };

        assert_eq!(pool.fee_bps(), 200); // 0.2% = 200 bps
    }

    #[test]
    fn test_valid_tick_data() {
        let valid_pool = ShyftPoolData {
            pubkey: "test".to_string(),
            token_mint_a: "mint_a".to_string(),
            token_mint_b: "mint_b".to_string(),
            fee_rate: 20000,
            tick_current_index: 1000,
            sqrt_price: "1000000".to_string(),
            liquidity: Some("1000000".to_string()),
        };

        let invalid_pool = ShyftPoolData {
            pubkey: "test".to_string(),
            token_mint_a: "mint_a".to_string(),
            token_mint_b: "mint_b".to_string(),
            fee_rate: 20000,
            tick_current_index: 0,
            sqrt_price: "".to_string(),
            liquidity: Some("1000000".to_string()),
        };

        assert!(valid_pool.has_valid_tick_data());
        assert!(!invalid_pool.has_valid_tick_data());
    }
}
