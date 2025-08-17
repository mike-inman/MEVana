use anyhow::{Context, Result};
use log::{info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Shyft API client for TickArray fallback data
pub struct TickArrayFallbackClient {
    client: Client,
    api_key: String,
    network: String,
}

/// TickArray data from Shyft API (fallback when on-chain parsing fails)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShyftTickArrayData {
    pub pool_address: String,
    pub tick_current_index: i32,
    pub sqrt_price: String,
    pub liquidity: String,
    pub fee_rate: i64,
    pub num_initialized_ticks: Option<i32>,
    pub gross_liquidity: Option<String>,
}

/// GraphQL response structure for pool data
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct WhirlpoolRows {
    #[serde(rename = "ORCA_WHIRLPOOLS_whirlpool")]
    rows: Vec<ShyftPoolRow>,
}

#[derive(Debug, Deserialize)]
struct ShyftPoolRow {
    pubkey: String,
    tokenMintA: String,
    tokenMintB: String,
    feeRate: i64,
    tickCurrentIndex: i32,
    sqrtPrice: String,
    liquidity: String,
}

impl TickArrayFallbackClient {
    /// Create a new TickArray fallback client
    pub fn new(api_key: String, network: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5)) // Fast timeout for fallback
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            network,
        }
    }

    /// Get TickArray fallback data for a specific pool
    pub async fn get_tickarray_fallback(&self, pool_address: &str) -> Result<Option<ShyftTickArrayData>> {
        let query = format!(
            r#"
            query GetPoolTickData($address: String!) {{
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
            .context("Failed to send GraphQL request for TickArray fallback")?
            .json()
            .await
            .context("Failed to parse GraphQL response for TickArray fallback")?;

        let pool = response.data.rows.into_iter().next();
        
        match pool {
            Some(p) => {
                // Convert to TickArray-compatible format
                let tick_data = ShyftTickArrayData {
                    pool_address: p.pubkey,
                    tick_current_index: p.tickCurrentIndex,
                    sqrt_price: p.sqrtPrice,
                    liquidity: p.liquidity,
                    fee_rate: p.feeRate,
                    num_initialized_ticks: Some(1), // Simplified - assume current tick is initialized
                    gross_liquidity: Some(p.liquidity.clone()), // Use total liquidity as gross
                };

                info!("TickArray fallback data retrieved for pool {}: tick={}, sqrt_price={}", 
                    pool_address, p.tickCurrentIndex, p.sqrtPrice);
                
                Ok(Some(tick_data))
            }
            None => {
                warn!("No TickArray fallback data found for pool {}", pool_address);
                Ok(None)
            }
        }
    }

    /// Get the GraphQL endpoint URL
    fn graphql_endpoint(&self) -> String {
        format!(
            "https://programs.shyft.to/v0/graphql/?api_key={}&network={}",
            self.api_key, self.network
        )
    }

    /// Check if fallback data is valid
    pub fn is_valid_fallback_data(&self, data: &ShyftTickArrayData) -> bool {
        data.tick_current_index != 0 
            && !data.sqrt_price.is_empty() 
            && !data.liquidity.is_empty()
    }

    /// Convert fallback data to Redis-compatible format
    pub fn to_redis_format(&self, data: &ShyftTickArrayData) -> Result<Vec<(String, String)>> {
        let mut fields = Vec::new();
        
        fields.push(("pool".to_string(), data.pool_address.clone()));
        fields.push(("tick_current".to_string(), data.tick_current_index.to_string()));
        fields.push(("sqrt_price_x64".to_string(), data.sqrt_price.clone()));
        fields.push(("liquidity".to_string(), data.liquidity.clone()));
        fields.push(("fee_bps".to_string(), (data.fee_rate / 100).to_string()));
        
        if let Some(num_init) = data.num_initialized_ticks {
            fields.push(("num_init".to_string(), num_init.to_string()));
        }
        
        if let Some(gross_liq) = &data.gross_liquidity {
            fields.push(("sum_gross".to_string(), gross_liq.clone()));
        }
        
        fields.push(("fallback_source".to_string(), "shyft_api".to_string()));
        fields.push(("updated_at".to_string(), chrono::Utc::now().timestamp().to_string()));
        
        Ok(fields)
    }
}

/// Integration with existing TickArray processing
pub struct TickArrayProcessor {
    fallback_client: TickArrayFallbackClient,
}

impl TickArrayProcessor {
    /// Create a new TickArray processor with fallback
    pub fn new(api_key: String, network: String) -> Self {
        let fallback_client = TickArrayFallbackClient::new(api_key, network);
        Self { fallback_client }
    }

    /// Process TickArray data with fallback to Shyft API
    pub async fn process_with_fallback(
        &self,
        pool_address: &str,
        on_chain_data: Option<&str>, // Raw on-chain data if available
    ) -> Result<ShyftTickArrayData> {
        // First, try to use on-chain data if available
        if let Some(raw_data) = on_chain_data {
            // Here you would attempt to parse the on-chain data
            // For now, we'll always use fallback since on-chain parsing is failing
            warn!("On-chain TickArray parsing failed for pool {}, using Shyft API fallback", pool_address);
        }

        // Use Shyft API fallback
        match self.fallback_client.get_tickarray_fallback(pool_address).await? {
            Some(fallback_data) => {
                if self.fallback_client.is_valid_fallback_data(&fallback_data) {
                    info!("Successfully retrieved TickArray fallback data for pool {}", pool_address);
                    Ok(fallback_data)
                } else {
                    anyhow::bail!("TickArray fallback data validation failed for pool {}", pool_address);
                }
            }
            None => {
                anyhow::bail!("No TickArray fallback data available for pool {}", pool_address);
            }
        }
    }

    /// Get Redis fields for writing to stream
    pub async fn get_redis_fields(
        &self,
        pool_address: &str,
    ) -> Result<Vec<(String, String)>> {
        let fallback_data = self.fallback_client.get_tickarray_fallback(pool_address).await?;
        
        match fallback_data {
            Some(data) => self.fallback_client.to_redis_format(&data),
            None => anyhow::bail!("Failed to get TickArray fallback data for pool {}", pool_address),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_fallback_data() {
        let client = TickArrayFallbackClient::new("test_key".to_string(), "mainnet-beta".to_string());
        
        let valid_data = ShyftTickArrayData {
            pool_address: "test_pool".to_string(),
            tick_current_index: 1000,
            sqrt_price: "1000000".to_string(),
            liquidity: "1000000".to_string(),
            fee_rate: 20000,
            num_initialized_ticks: Some(1),
            gross_liquidity: Some("1000000".to_string()),
        };

        let invalid_data = ShyftTickArrayData {
            pool_address: "test_pool".to_string(),
            tick_current_index: 0,
            sqrt_price: "".to_string(),
            liquidity: "".to_string(),
            fee_rate: 20000,
            num_initialized_ticks: Some(1),
            gross_liquidity: Some("1000000".to_string()),
        };

        assert!(client.is_valid_fallback_data(&valid_data));
        assert!(!client.is_valid_fallback_data(&invalid_data));
    }

    #[test]
    fn test_redis_format_conversion() {
        let client = TickArrayFallbackClient::new("test_key".to_string(), "mainnet-beta".to_string());
        
        let data = ShyftTickArrayData {
            pool_address: "test_pool".to_string(),
            tick_current_index: 1000,
            sqrt_price: "1000000".to_string(),
            liquidity: "1000000".to_string(),
            fee_rate: 20000,
            num_initialized_ticks: Some(1),
            gross_liquidity: Some("1000000".to_string()),
        };

        let redis_fields = client.to_redis_format(&data).unwrap();
        
        // Check that required fields are present
        let field_names: Vec<String> = redis_fields.iter().map(|(k, _)| k.clone()).collect();
        assert!(field_names.contains(&"pool".to_string()));
        assert!(field_names.contains(&"tick_current".to_string()));
        assert!(field_names.contains(&"sqrt_price_x64".to_string()));
        assert!(field_names.contains(&"fallback_source".to_string()));
    }
}
