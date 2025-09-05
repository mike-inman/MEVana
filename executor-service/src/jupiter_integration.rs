use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// Jupiter API v6 structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JupiterQuoteRequest {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    pub amount: String,
    #[serde(rename = "slippageBps", skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u16>,
    #[serde(rename = "feeBps", skip_serializing_if = "Option::is_none")]
    pub fee_bps: Option<u16>,
    #[serde(rename = "onlyDirectRoutes", skip_serializing_if = "Option::is_none")]
    pub only_direct_routes: Option<bool>,
    #[serde(rename = "asLegacyTransaction", skip_serializing_if = "Option::is_none")]
    pub as_legacy_transaction: Option<bool>,
    #[serde(rename = "maxAccounts", skip_serializing_if = "Option::is_none")]
    pub max_accounts: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee", skip_serializing_if = "Option::is_none")]
    pub platform_fee: Option<PlatformFee>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlanStep>,
    #[serde(rename = "contextSlot")]
    pub context_slot: u64,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoutePlanStep {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformFee {
    pub amount: String,
    #[serde(rename = "feeBps")]
    pub fee_bps: u16,
}

#[derive(Debug, Clone)]
pub struct MultiHopOpportunity {
    pub opportunity_id: String,
    pub route: Vec<HopStep>,
    pub input_token: String,
    pub output_token: String,
    pub input_amount: u64,
    pub expected_output: u64,
    pub minimum_output: u64,
    pub total_fees: u64,
    pub price_impact_pct: f64,
    pub profit_bps: i32,
    pub route_complexity: u8, // Number of hops
    pub venues_used: Vec<String>,
    pub execution_time_estimate: Duration,
    pub detected_at: Instant,
    pub expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct HopStep {
    pub from_token: String,
    pub to_token: String,
    pub venue: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub price_impact: f64,
}

pub struct JupiterIntegration {
    client: Client,
    base_url: String,
    request_timeout: Duration,
    max_slippage_bps: u16,
    max_price_impact: f64,
    min_profit_bps: i32,
    cache: HashMap<String, (JupiterQuoteResponse, Instant)>,
    cache_ttl: Duration,
    metrics: JupiterMetrics,
}

#[derive(Debug, Default)]
struct JupiterMetrics {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    cache_hits: u64,
    cache_misses: u64,
    opportunities_found: u64,
    multi_hop_opportunities: u64,
    average_response_time_ms: f64,
}

impl JupiterIntegration {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("MEVana/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://quote-api.jup.ag".to_string(),
            request_timeout: Duration::from_secs(5),
            max_slippage_bps: 300, // 3%
            max_price_impact: 5.0, // 5%
            min_profit_bps: 25, // 0.25%
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(2),
            metrics: JupiterMetrics::default(),
        }
    }

    pub fn with_config(
        max_slippage_bps: u16,
        max_price_impact: f64,
        min_profit_bps: i32,
    ) -> Self {
        let mut instance = Self::new();
        instance.max_slippage_bps = max_slippage_bps;
        instance.max_price_impact = max_price_impact;
        instance.min_profit_bps = min_profit_bps;
        instance
    }

    /// Get Jupiter quote for a direct swap
    pub async fn get_quote(
        &mut self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: Option<u16>,
    ) -> Result<JupiterQuoteResponse> {
        let cache_key = format!("{}:{}:{}", input_mint, output_mint, amount);
        
        // Check cache first
        if let Some((cached_quote, cached_at)) = self.cache.get(&cache_key) {
            if cached_at.elapsed() < self.cache_ttl {
                self.metrics.cache_hits += 1;
                debug!(
                    "Cache hit for Jupiter quote: {} -> {} ({})",
                    input_mint, output_mint, amount
                );
                return Ok(cached_quote.clone());
            }
        }
        self.metrics.cache_misses += 1;

        let start_time = Instant::now();
        self.metrics.total_requests += 1;

        let request = JupiterQuoteRequest {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount: amount.to_string(),
            slippage_bps,
            fee_bps: None,
            only_direct_routes: Some(false), // Allow multi-hop routes
            as_legacy_transaction: Some(false),
            max_accounts: Some(64), // Allow complex routes
        };

        let url = format!("{}/v6/quote", self.base_url);
        
        info!(
            "🔍 Getting Jupiter quote: {} -> {} (amount: {})",
            input_mint, output_mint, amount
        );

        let response = timeout(
            self.request_timeout,
            self.client.get(&url).query(&request).send(),
        )
        .await
        .context("Request timeout")?
        .context("Failed to send request")?;

        let response_time = start_time.elapsed();
        self.update_response_time_metric(response_time.as_millis() as f64);

        if response.status().is_success() {
            let quote: JupiterQuoteResponse = response
                .json()
                .await
                .context("Failed to parse Jupiter response")?;

            self.metrics.successful_requests += 1;

            // Cache the response
            self.cache.insert(cache_key, (quote.clone(), Instant::now()));
            
            info!(
                "✅ Jupiter quote received: {} -> {} ({} hops, {:.4}% price impact)",
                input_mint,
                output_mint,
                quote.route_plan.len(),
                quote.price_impact_pct.parse::<f64>().unwrap_or(0.0)
            );

            Ok(quote)
        } else {
            self.metrics.failed_requests += 1;
            let error_text = response.text().await.unwrap_or_default();
            error!("Jupiter API error: {}", error_text);
            anyhow::bail!("Jupiter API error: {}", error_text);
        }
    }

    /// Find multi-hop arbitrage opportunities using Jupiter routing
    pub async fn find_multi_hop_opportunities(
        &mut self,
        tokens: &[String],
        base_amount: u64,
    ) -> Result<Vec<MultiHopOpportunity>> {
        let mut opportunities = Vec::new();
        let start_time = Instant::now();

        info!(
            "🔍 Searching for multi-hop opportunities across {} tokens with base amount {}",
            tokens.len(),
            base_amount
        );

        // Generate all possible 2-hop, 3-hop, and 4-hop cycles
        for start_token in tokens {
            // 2-hop: A -> B -> A
            for intermediate in tokens {
                if intermediate == start_token {
                    continue;
                }
                
                if let Ok(opportunity) = self.analyze_two_hop_cycle(
                    start_token,
                    intermediate,
                    base_amount,
                ).await {
                    if opportunity.profit_bps >= self.min_profit_bps {
                        opportunities.push(opportunity);
                    }
                }
            }

            // 3-hop: A -> B -> C -> A
            for first_intermediate in tokens {
                if first_intermediate == start_token {
                    continue;
                }
                
                for second_intermediate in tokens {
                    if second_intermediate == start_token 
                        || second_intermediate == first_intermediate {
                        continue;
                    }

                    if let Ok(opportunity) = self.analyze_three_hop_cycle(
                        start_token,
                        first_intermediate,
                        second_intermediate,
                        base_amount,
                    ).await {
                        if opportunity.profit_bps >= self.min_profit_bps {
                            opportunities.push(opportunity);
                        }
                    }
                }
            }
        }

        // Sort by profit (descending)
        opportunities.sort_by(|a, b| b.profit_bps.cmp(&a.profit_bps));

        self.metrics.opportunities_found += opportunities.len() as u64;
        self.metrics.multi_hop_opportunities += opportunities.iter()
            .filter(|op| op.route_complexity > 2)
            .count() as u64;

        info!(
            "✅ Found {} multi-hop opportunities in {:.2}s",
            opportunities.len(),
            start_time.elapsed().as_secs_f64()
        );

        // Log top opportunities
        for (i, opp) in opportunities.iter().take(5).enumerate() {
            info!(
                "#{} Multi-hop opportunity: {} hops, {}bps profit, venues: {:?}",
                i + 1,
                opp.route_complexity,
                opp.profit_bps,
                opp.venues_used
            );
        }

        Ok(opportunities)
    }

    /// Analyze a 2-hop arbitrage cycle: A -> B -> A
    async fn analyze_two_hop_cycle(
        &mut self,
        start_token: &str,
        intermediate_token: &str,
        amount: u64,
    ) -> Result<MultiHopOpportunity> {
        // Step 1: A -> B
        let quote1 = self.get_quote(
            start_token,
            intermediate_token,
            amount,
            Some(self.max_slippage_bps),
        ).await?;

        let intermediate_amount: u64 = quote1.out_amount.parse()
            .context("Failed to parse intermediate amount")?;

        // Step 2: B -> A  
        let quote2 = self.get_quote(
            intermediate_token,
            start_token,
            intermediate_amount,
            Some(self.max_slippage_bps),
        ).await?;

        let final_amount: u64 = quote2.out_amount.parse()
            .context("Failed to parse final amount")?;

        // Calculate profit
        let profit = final_amount as i64 - amount as i64;
        let profit_bps = (profit * 10000) / amount as i64;

        // Calculate total fees and price impact
        let total_price_impact = quote1.price_impact_pct.parse::<f64>().unwrap_or(0.0)
            + quote2.price_impact_pct.parse::<f64>().unwrap_or(0.0);

        let total_fees = self.calculate_total_fees(&quote1) + self.calculate_total_fees(&quote2);

        let route = vec![
            HopStep {
                from_token: start_token.to_string(),
                to_token: intermediate_token.to_string(),
                venue: self.extract_primary_venue(&quote1),
                amount_in: amount,
                amount_out: intermediate_amount,
                fee_amount: self.calculate_total_fees(&quote1),
                price_impact: quote1.price_impact_pct.parse().unwrap_or(0.0),
            },
            HopStep {
                from_token: intermediate_token.to_string(),
                to_token: start_token.to_string(),
                venue: self.extract_primary_venue(&quote2),
                amount_in: intermediate_amount,
                amount_out: final_amount,
                fee_amount: self.calculate_total_fees(&quote2),
                price_impact: quote2.price_impact_pct.parse().unwrap_or(0.0),
            },
        ];

        let venues_used = route.iter()
            .map(|step| step.venue.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(MultiHopOpportunity {
            opportunity_id: format!(
                "2hop_{}_{}_{}",
                start_token,
                intermediate_token,
                chrono::Utc::now().timestamp_millis()
            ),
            route,
            input_token: start_token.to_string(),
            output_token: start_token.to_string(),
            input_amount: amount,
            expected_output: final_amount,
            minimum_output: (final_amount as f64 * 0.99) as u64, // 1% slippage buffer
            total_fees,
            price_impact_pct: total_price_impact,
            profit_bps: profit_bps as i32,
            route_complexity: 2,
            venues_used,
            execution_time_estimate: Duration::from_millis(1500), // Estimate for 2 swaps
            detected_at: Instant::now(),
            expires_at: Instant::now() + Duration::from_secs(30), // Opportunities expire quickly
        })
    }

    /// Analyze a 3-hop arbitrage cycle: A -> B -> C -> A
    async fn analyze_three_hop_cycle(
        &mut self,
        start_token: &str,
        first_intermediate: &str,
        second_intermediate: &str,
        amount: u64,
    ) -> Result<MultiHopOpportunity> {
        // Step 1: A -> B
        let quote1 = self.get_quote(
            start_token,
            first_intermediate,
            amount,
            Some(self.max_slippage_bps),
        ).await?;

        let first_amount: u64 = quote1.out_amount.parse()?;

        // Step 2: B -> C
        let quote2 = self.get_quote(
            first_intermediate,
            second_intermediate,
            first_amount,
            Some(self.max_slippage_bps),
        ).await?;

        let second_amount: u64 = quote2.out_amount.parse()?;

        // Step 3: C -> A
        let quote3 = self.get_quote(
            second_intermediate,
            start_token,
            second_amount,
            Some(self.max_slippage_bps),
        ).await?;

        let final_amount: u64 = quote3.out_amount.parse()?;

        // Calculate profit and metrics
        let profit = final_amount as i64 - amount as i64;
        let profit_bps = (profit * 10000) / amount as i64;

        let total_price_impact = quote1.price_impact_pct.parse::<f64>().unwrap_or(0.0)
            + quote2.price_impact_pct.parse::<f64>().unwrap_or(0.0)
            + quote3.price_impact_pct.parse::<f64>().unwrap_or(0.0);

        let total_fees = self.calculate_total_fees(&quote1) 
            + self.calculate_total_fees(&quote2)
            + self.calculate_total_fees(&quote3);

        let route = vec![
            HopStep {
                from_token: start_token.to_string(),
                to_token: first_intermediate.to_string(),
                venue: self.extract_primary_venue(&quote1),
                amount_in: amount,
                amount_out: first_amount,
                fee_amount: self.calculate_total_fees(&quote1),
                price_impact: quote1.price_impact_pct.parse().unwrap_or(0.0),
            },
            HopStep {
                from_token: first_intermediate.to_string(),
                to_token: second_intermediate.to_string(),
                venue: self.extract_primary_venue(&quote2),
                amount_in: first_amount,
                amount_out: second_amount,
                fee_amount: self.calculate_total_fees(&quote2),
                price_impact: quote2.price_impact_pct.parse().unwrap_or(0.0),
            },
            HopStep {
                from_token: second_intermediate.to_string(),
                to_token: start_token.to_string(),
                venue: self.extract_primary_venue(&quote3),
                amount_in: second_amount,
                amount_out: final_amount,
                fee_amount: self.calculate_total_fees(&quote3),
                price_impact: quote3.price_impact_pct.parse().unwrap_or(0.0),
            },
        ];

        let venues_used = route.iter()
            .map(|step| step.venue.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(MultiHopOpportunity {
            opportunity_id: format!(
                "3hop_{}_{}_{}_{}",
                start_token,
                first_intermediate,
                second_intermediate,
                chrono::Utc::now().timestamp_millis()
            ),
            route,
            input_token: start_token.to_string(),
            output_token: start_token.to_string(),
            input_amount: amount,
            expected_output: final_amount,
            minimum_output: (final_amount as f64 * 0.98) as u64, // 2% slippage buffer for 3-hop
            total_fees,
            price_impact_pct: total_price_impact,
            profit_bps: profit_bps as i32,
            route_complexity: 3,
            venues_used,
            execution_time_estimate: Duration::from_millis(2500), // Estimate for 3 swaps
            detected_at: Instant::now(),
            expires_at: Instant::now() + Duration::from_secs(20), // Shorter expiry for complex routes
        })
    }

    /// Extract primary venue from Jupiter route plan
    fn extract_primary_venue(&self, quote: &JupiterQuoteResponse) -> String {
        quote.route_plan
            .first()
            .map(|step| step.swap_info.label.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Calculate total fees from Jupiter quote
    fn calculate_total_fees(&self, quote: &JupiterQuoteResponse) -> u64 {
        quote.route_plan
            .iter()
            .map(|step| {
                step.swap_info.fee_amount.parse::<u64>().unwrap_or(0)
            })
            .sum()
    }

    /// Update average response time metric
    fn update_response_time_metric(&mut self, response_time_ms: f64) {
        let current_avg = self.metrics.average_response_time_ms;
        let count = self.metrics.total_requests as f64;
        self.metrics.average_response_time_ms = 
            (current_avg * (count - 1.0) + response_time_ms) / count;
    }

    /// Clean expired cache entries
    pub fn cleanup_cache(&mut self) {
        let now = Instant::now();
        self.cache.retain(|_, (_, cached_at)| {
            now.duration_since(*cached_at) < self.cache_ttl
        });
    }

    /// Get metrics for monitoring
    pub fn get_metrics(&self) -> &JupiterMetrics {
        &self.metrics
    }

    /// Reset metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = JupiterMetrics::default();
    }
}

// Common Solana token mints for multi-hop opportunities
pub const COMMON_TOKENS: &[&str] = &[
    "So11111111111111111111111111111111111111112", // SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // Bonk
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",  // mSOL
    "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1",  // bSOL
    "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs", // ETH (Wormhole)
    "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E", // BTC (Wormhole)
];

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jupiter_quote() {
        let mut jupiter = JupiterIntegration::new();
        
        // Test SOL -> USDC quote
        let result = jupiter.get_quote(
            "So11111111111111111111111111111111111111112", // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
            1_000_000_000, // 1 SOL
            Some(100), // 1% slippage
        ).await;

        assert!(result.is_ok());
        let quote = result.unwrap();
        assert!(!quote.out_amount.is_empty());
        assert!(!quote.route_plan.is_empty());
    }

    #[tokio::test] 
    async fn test_multi_hop_detection() {
        let mut jupiter = JupiterIntegration::new();
        
        let tokens = vec![
            "So11111111111111111111111111111111111111112".to_string(), // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
            "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So".to_string(), // mSOL
        ];

        let opportunities = jupiter.find_multi_hop_opportunities(
            &tokens,
            1_000_000_000, // 1 SOL
        ).await;

        assert!(opportunities.is_ok());
        // Should find some opportunities in current market conditions
    }
}