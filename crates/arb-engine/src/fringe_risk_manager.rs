// crates/arb-engine/src/fringe_risk_manager.rs
//! Risk management for fringe token arbitrage opportunities
//! Prevents losses from low-liquidity, high-volatility tokens

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRiskProfile {
    pub mint: String,
    pub symbol: String,
    pub current_liquidity_sol: f64,
    pub volatility_24h: f64,
    pub price_impact_5_sol: f64,
    pub last_updated: u64,
    pub risk_score: f64, // 0.0 = safe, 1.0 = extremely risky
    pub category: TokenCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenCategory {
    Core,        // SOL, USDC, USDT
    HighVolume,  // BONK, mSOL, ETH
    Meme,        // WIF, CAT, GOAT
    Gaming,      // ATLAS, POLIS  
    DePIN,       // HNT, IOT, MOBILE
    Infrastructure, // jitoSOL, LIDO
    Bridge,      // axlUSDC, mUSD
    Experimental, // New/unknown tokens
}

#[derive(Debug, Clone)]
pub struct FringeRiskConfig {
    pub max_position_sol: f64,
    pub min_liquidity_sol: f64,
    pub max_volatility: f64,
    pub max_price_impact: f64,
    pub max_risk_score: f64,
}

impl Default for FringeRiskConfig {
    fn default() -> Self {
        Self {
            max_position_sol: std::env::var("FRINGE_MAX_POSITION_SOL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2.0),
            min_liquidity_sol: std::env::var("FRINGE_MIN_LIQUIDITY_SOL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10.0),
            max_volatility: std::env::var("FRINGE_VOLATILITY_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.15), // 15%
            max_price_impact: 0.05, // 5% max price impact
            max_risk_score: 0.7,    // 70% max risk score
        }
    }
}

pub struct FringeRiskManager {
    config: FringeRiskConfig,
    token_profiles: Arc<RwLock<HashMap<String, TokenRiskProfile>>>,
    position_tracker: Arc<RwLock<HashMap<String, f64>>>, // mint -> current_position_sol
}

impl FringeRiskManager {
    pub fn new() -> Self {
        let config = FringeRiskConfig::default();
        info!(
            "🛡️ FringeRiskManager initialized: max_pos={}SOL, min_liq={}SOL, max_vol={}%",
            config.max_position_sol,
            config.min_liquidity_sol,
            config.max_volatility * 100.0
        );

        Self {
            config,
            token_profiles: Arc::new(RwLock::new(HashMap::new())),
            position_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a fringe token trade should be approved
    pub async fn approve_trade(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount_sol: f64,
        expected_profit_bps: i32,
    ) -> Result<TradeApproval> {
        // Get risk profiles for both tokens
        let input_risk = self.get_or_create_risk_profile(input_mint).await?;
        let output_risk = self.get_or_create_risk_profile(output_mint).await?;

        // Calculate combined risk score
        let combined_risk = (input_risk.risk_score + output_risk.risk_score) / 2.0;

        // Check position limits
        let current_positions = self.position_tracker.read().await;
        let input_position = current_positions.get(input_mint).unwrap_or(&0.0);
        let output_position = current_positions.get(output_mint).unwrap_or(&0.0);

        // Risk checks
        let mut reasons = Vec::new();

        // 1. Position size limits
        if input_position + amount_sol > self.config.max_position_sol {
            reasons.push(format!(
                "Input position limit exceeded: {} + {} > {}",
                input_position, amount_sol, self.config.max_position_sol
            ));
        }

        // 2. Liquidity requirements
        if input_risk.current_liquidity_sol < self.config.min_liquidity_sol {
            reasons.push(format!(
                "Input liquidity too low: {} < {}",
                input_risk.current_liquidity_sol, self.config.min_liquidity_sol
            ));
        }
        if output_risk.current_liquidity_sol < self.config.min_liquidity_sol {
            reasons.push(format!(
                "Output liquidity too low: {} < {}",
                output_risk.current_liquidity_sol, self.config.min_liquidity_sol
            ));
        }

        // 3. Volatility limits
        if input_risk.volatility_24h > self.config.max_volatility {
            reasons.push(format!(
                "Input volatility too high: {}% > {}%",
                input_risk.volatility_24h * 100.0,
                self.config.max_volatility * 100.0
            ));
        }

        // 4. Risk score limits
        if combined_risk > self.config.max_risk_score {
            reasons.push(format!(
                "Combined risk score too high: {} > {}",
                combined_risk, self.config.max_risk_score
            ));
        }

        // 5. Minimum profit requirements for risky tokens
        let min_profit_bps = match combined_risk {
            r if r > 0.6 => 25, // High risk requires 25+ BPS
            r if r > 0.4 => 15, // Medium risk requires 15+ BPS
            _ => 8,             // Low risk can use default 8 BPS
        };

        if expected_profit_bps < min_profit_bps {
            reasons.push(format!(
                "Insufficient profit for risk level: {}bps < {}bps (risk: {:.2})",
                expected_profit_bps, min_profit_bps, combined_risk
            ));
        }

        let approval = if reasons.is_empty() {
            TradeApproval::Approved {
                adjusted_amount_sol: amount_sol.min(self.config.max_position_sol - input_position),
                risk_score: combined_risk,
                required_profit_bps: min_profit_bps,
            }
        } else {
            TradeApproval::Rejected { reasons }
        };

        debug!(
            "🛡️ Trade approval for {}->{}: {:?}",
            input_risk.symbol, output_risk.symbol, approval
        );

        Ok(approval)
    }

    /// Update position after trade execution
    pub async fn update_position(&self, mint: &str, amount_change_sol: f64) -> Result<()> {
        let mut positions = self.position_tracker.write().await;
        let current = positions.get(mint).unwrap_or(&0.0);
        let new_position = (current + amount_change_sol).max(0.0);
        
        positions.insert(mint.to_string(), new_position);
        
        debug!("📊 Position updated for {}: {:.3} SOL", mint, new_position);
        Ok(())
    }

    /// Get or create risk profile for a token
    async fn get_or_create_risk_profile(&self, mint: &str) -> Result<TokenRiskProfile> {
        {
            let profiles = self.token_profiles.read().await;
            if let Some(profile) = profiles.get(mint) {
                // Check if profile is fresh (less than 5 minutes old)
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                if now - profile.last_updated < 300 {
                    return Ok(profile.clone());
                }
            }
        }

        // Create or refresh profile
        let profile = self.create_risk_profile(mint).await?;
        
        {
            let mut profiles = self.token_profiles.write().await;
            profiles.insert(mint.to_string(), profile.clone());
        }
        
        Ok(profile)
    }

    /// Create a risk profile for a token (mock implementation)
    async fn create_risk_profile(&self, mint: &str) -> Result<TokenRiskProfile> {
        // In a real implementation, this would query:
        // - Jupiter API for current pools and liquidity
        // - Price feeds for volatility data
        // - Token metadata for symbol and category

        let (symbol, category, base_risk) = self.classify_token(mint);
        
        // Mock data for now - in production this would be real API calls
        let profile = TokenRiskProfile {
            mint: mint.to_string(),
            symbol,
            current_liquidity_sol: match category {
                TokenCategory::Core => 50000.0,
                TokenCategory::HighVolume => 10000.0,
                TokenCategory::Meme => 1000.0,
                TokenCategory::Gaming => 500.0,
                TokenCategory::DePIN => 2000.0,
                TokenCategory::Infrastructure => 5000.0,
                TokenCategory::Bridge => 3000.0,
                TokenCategory::Experimental => 100.0,
            },
            volatility_24h: base_risk * 0.1, // Convert risk to volatility
            price_impact_5_sol: base_risk * 0.02, // 2% max for high risk
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            risk_score: base_risk,
            category,
        };

        info!(
            "📊 Created risk profile for {}: {} (risk: {:.2}, liq: {:.0} SOL)",
            profile.symbol, profile.mint, profile.risk_score, profile.current_liquidity_sol
        );

        Ok(profile)
    }

    /// Classify a token by mint address
    fn classify_token(&self, mint: &str) -> (String, TokenCategory, f64) {
        match mint {
            // Core tokens (lowest risk)
            "So11111111111111111111111111111111111111112" => ("SOL".to_string(), TokenCategory::Core, 0.1),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => ("USDC".to_string(), TokenCategory::Core, 0.05),
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => ("USDT".to_string(), TokenCategory::Core, 0.05),
            
            // High volume tokens (low-medium risk)
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => ("BONK".to_string(), TokenCategory::HighVolume, 0.3),
            "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => ("mSOL".to_string(), TokenCategory::HighVolume, 0.15),
            
            // Meme tokens (high risk)
            "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3npgxnCJcmCPFG" => ("CAT".to_string(), TokenCategory::Meme, 0.7),
            "3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh" => ("WIF".to_string(), TokenCategory::Meme, 0.8),
            
            // Gaming tokens (medium-high risk)
            "ATLASXmbPQxBUYbxPsV97usA3fPQYEqzQBUHgiFCUsXx" => ("ATLAS".to_string(), TokenCategory::Gaming, 0.5),
            
            // DePIN tokens (medium risk)
            "HeLp6NuQkmYB4pYWo2zYs22mESHXPQYzXbB8n4V98jwC" => ("HNT".to_string(), TokenCategory::DePIN, 0.4),
            
            // Default: experimental (highest risk)
            _ => ("UNKNOWN".to_string(), TokenCategory::Experimental, 0.9),
        }
    }

    /// Get current risk statistics
    pub async fn get_risk_stats(&self) -> RiskStats {
        let profiles = self.token_profiles.read().await;
        let positions = self.position_tracker.read().await;

        RiskStats {
            total_tokens_tracked: profiles.len(),
            total_position_value_sol: positions.values().sum(),
            high_risk_tokens: profiles.values().filter(|p| p.risk_score > 0.6).count(),
            avg_risk_score: profiles.values().map(|p| p.risk_score).sum::<f64>() / profiles.len() as f64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeApproval {
    Approved {
        adjusted_amount_sol: f64,
        risk_score: f64,
        required_profit_bps: i32,
    },
    Rejected {
        reasons: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskStats {
    pub total_tokens_tracked: usize,
    pub total_position_value_sol: f64,
    pub high_risk_tokens: usize,
    pub avg_risk_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fringe_risk_approval() {
        let risk_manager = FringeRiskManager::new();

        // Test core token (should be approved)
        let approval = risk_manager
            .approve_trade(
                "So11111111111111111111111111111111111111112", // SOL
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
                1.0,
                10,
            )
            .await
            .unwrap();

        match approval {
            TradeApproval::Approved { .. } => (),
            TradeApproval::Rejected { reasons } => panic!("Core token trade rejected: {:?}", reasons),
        }

        // Test experimental token with low profit (should be rejected)
        let approval = risk_manager
            .approve_trade(
                "UnknownTokenMint123456789", // Unknown token
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
                1.0,
                5, // Low profit for high risk
            )
            .await
            .unwrap();

        match approval {
            TradeApproval::Rejected { .. } => (),
            TradeApproval::Approved { .. } => panic!("High risk/low profit trade approved"),
        }
    }
}