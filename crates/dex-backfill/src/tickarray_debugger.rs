use anyhow::{Context, Result};
use log::{info, warn, error};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// TickArray debugger using Shyft APIs to understand parsing failures
pub struct TickArrayDebugger {
    client: Client,
    api_key: String,
    network: String,
}

/// Debug information about a TickArray parsing failure
#[derive(Debug, Serialize, Clone)]
pub struct TickArrayDebugInfo {
    pub pool_address: String,
    pub expected_tick_data: Option<ShyftTickData>,
    pub raw_data_analysis: RawDataAnalysis,
    pub parsing_insights: Vec<String>,
}

/// Expected tick data from Shyft API
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShyftTickData {
    pub tick_current_index: i32,
    pub sqrt_price: String,
    pub liquidity: String,
    pub fee_rate: i64,
}

/// Analysis of the raw account data that failed to parse
#[derive(Debug, Serialize, Clone)]
pub struct RawDataAnalysis {
    pub data_length: usize,
    pub first_bytes: Vec<u8>,
    pub last_bytes: Vec<u8>,
    pub data_hex: String,
    pub potential_discriminator: Vec<u8>,
}

/// GraphQL response structure
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

impl TickArrayDebugger {
    /// Create a new TickArray debugger
    pub fn new(api_key: String, network: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10)) // Longer timeout for debugging
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            network,
        }
    }

    /// Debug a TickArray parsing failure by comparing with Shyft data
    pub async fn debug_parsing_failure(
        &self,
        pool_address: &str,
        raw_account_data: &[u8],
        parsing_error: &str,
    ) -> Result<TickArrayDebugInfo> {
        info!("🔍 Debugging TickArray parsing failure for pool {}", pool_address);
        
        // Get expected data from Shyft API
        let expected_data = self.get_pool_tick_data(pool_address).await?;
        
        // Analyze the raw data that failed to parse
        let raw_analysis = self.analyze_raw_data(raw_account_data);
        
        // Generate insights about why parsing failed
        let insights = self.generate_parsing_insights(
            &expected_data,
            &raw_analysis,
            parsing_error,
        );

        let debug_info = TickArrayDebugInfo {
            pool_address: pool_address.to_string(),
            expected_tick_data: expected_data,
            raw_data_analysis: raw_analysis,
            parsing_insights: insights,
        };

        info!("📊 Debug analysis complete for pool {}", pool_address);
        Ok(debug_info)
    }

    /// Get pool tick data from Shyft API for comparison
    async fn get_pool_tick_data(&self, pool_address: &str) -> Result<Option<ShyftTickData>> {
        let query = format!(
            r#"
            query GetPoolTickData($address: String!) {{
                ORCA_WHIRLPOOLS_whirlpool(
                    where: {{ pubkey: {{ _eq: $address }} }}
                    limit: 1
                ) {{
                    tickCurrentIndex
                    sqrtPrice
                    liquidity
                    feeRate
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
            .context("Failed to send GraphQL request for debugging")?
            .json()
            .await
            .context("Failed to parse GraphQL response for debugging")?;

        let pool = response.data.rows.into_iter().next();
        
        match pool {
            Some(p) => {
                let tick_data = ShyftTickData {
                    tick_current_index: p.tickCurrentIndex,
                    sqrt_price: p.sqrtPrice,
                    liquidity: p.liquidity,
                    fee_rate: p.feeRate,
                };
                Ok(Some(tick_data))
            }
            None => {
                warn!("No Shyft data found for pool {} during debugging", pool_address);
                Ok(None)
            }
        }
    }

    /// Analyze the raw account data that failed to parse
    fn analyze_raw_data(&self, data: &[u8]) -> RawDataAnalysis {
        let data_length = data.len();
        let first_bytes = data.iter().take(8).cloned().collect();
        let last_bytes = data.iter().rev().take(8).cloned().collect::<Vec<u8>>().into_iter().rev().collect();
        let data_hex = hex::encode(data);
        let potential_discriminator = data.iter().take(8).cloned().collect();

        RawDataAnalysis {
            data_length,
            first_bytes,
            last_bytes,
            data_hex,
            potential_discriminator,
        }
    }

    /// Generate insights about why parsing failed
    fn generate_parsing_insights(
        &self,
        expected_data: &Option<ShyftTickData>,
        raw_analysis: &RawDataAnalysis,
        parsing_error: &str,
    ) -> Vec<String> {
        let mut insights = Vec::new();

        // Analyze data length
        if raw_analysis.data_length < 8 {
            insights.push(format!("Data too short: {} bytes (expected at least 8 for discriminator)", raw_analysis.data_length));
        }

        // Check discriminator
        let expected_discriminator = [0x5b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b]; // TICK_ARRAY_ACCOUNT_DISCM
        if raw_analysis.potential_discriminator != expected_discriminator {
            insights.push(format!(
                "Unexpected discriminator: {:?} (expected {:?})", 
                raw_analysis.potential_discriminator, 
                expected_discriminator
            ));
        }

        // Analyze parsing error message
        if parsing_error.contains("Invalid bool representation") {
            insights.push("Bool parsing error suggests data structure mismatch".to_string());
            insights.push("Check if IDL matches actual on-chain data layout".to_string());
        }

        if parsing_error.contains("Invalid enum") {
            insights.push("Enum parsing error suggests variant mismatch".to_string());
        }

        if parsing_error.contains("Invalid length") {
            insights.push("Length error suggests data corruption or wrong account type".to_string());
        }

        // Compare with expected data if available
        if let Some(expected) = expected_data {
            insights.push(format!("Expected tick index: {}", expected.tick_current_index));
            insights.push(format!("Expected sqrt price: {}", expected.sqrt_price));
            insights.push(format!("Expected liquidity: {}", expected.liquidity));
        }

        // General debugging tips
        insights.push("Consider regenerating IDL with 'anchor idl fetch'".to_string());
        insights.push("Check if account is actually a TickArray (not a different account type)".to_string());
        insights.push("Verify program ID matches expected Whirlpool program".to_string());

        insights
    }

    /// Get the GraphQL endpoint URL
    fn graphql_endpoint(&self) -> String {
        format!(
            "https://programs.shyft.to/v0/graphql/?api_key={}&network={}",
            self.api_key, self.network
        )
    }

    /// Log comprehensive debug information
    pub fn log_debug_info(&self, debug_info: &TickArrayDebugInfo) {
        error!("🔍 TICKARRAY PARSING FAILURE DEBUG REPORT");
        error!("==========================================");
        error!("Pool: {}", debug_info.pool_address);
        
        if let Some(expected) = &debug_info.expected_tick_data {
            error!("Expected Data from Shyft:");
            error!("  Tick Index: {}", expected.tick_current_index);
            error!("  Sqrt Price: {}", expected.sqrt_price);
            error!("  Liquidity: {}", expected.liquidity);
            error!("  Fee Rate: {} bps", expected.fee_rate / 100);
        }
        
        error!("Raw Data Analysis:");
        error!("  Length: {} bytes", debug_info.raw_data_analysis.data_length);
        error!("  First 8 bytes: {:?}", debug_info.raw_data_analysis.first_bytes);
        error!("  Last 8 bytes: {:?}", debug_info.raw_data_analysis.last_bytes);
        error!("  Discriminator: {:?}", debug_info.raw_data_analysis.potential_discriminator);
        error!("  Full data (hex): {}", debug_info.raw_data_analysis.data_hex);
        
        error!("Parsing Insights:");
        for insight in &debug_info.parsing_insights {
            error!("  • {}", insight);
        }
        error!("==========================================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_data_analysis() {
        let debugger = TickArrayDebugger::new("test_key".to_string(), "mainnet-beta".to_string());
        
        let test_data = vec![0x5b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x01, 0x02, 0x03];
        let analysis = debugger.analyze_raw_data(&test_data);
        
        assert_eq!(analysis.data_length, 11);
        assert_eq!(analysis.first_bytes, vec![0x5b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b]);
        assert_eq!(analysis.potential_discriminator, vec![0x5b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b, 0x4b]);
    }

    #[test]
    fn test_parsing_insights() {
        let debugger = TickArrayDebugger::new("test_key".to_string(), "mainnet-beta".to_string());
        
        let raw_analysis = RawDataAnalysis {
            data_length: 5,
            first_bytes: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            last_bytes: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            data_hex: "0102030405".to_string(),
            potential_discriminator: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        };
        
        let insights = debugger.generate_parsing_insights(&None, &raw_analysis, "Invalid bool representation: 150");
        
        assert!(insights.iter().any(|s| s.contains("Data too short")));
        assert!(insights.iter().any(|s| s.contains("Bool parsing error")));
        assert!(insights.iter().any(|s| s.contains("regenerating IDL")));
    }
}
