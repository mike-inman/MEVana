use crate::opportunity::ArbitrageOpportunity;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct RiskManager {
    max_opportunities_per_second: u64,
    max_size_per_opportunity: u128,
    min_profit_threshold_bps: u64,
    max_edge_age_ms: u64,
    max_total_cu: u32,
    
    // Tracking (using Arc for shared atomic state)
    opportunities_this_second: Arc<AtomicU64>,
    last_reset_time: Arc<AtomicU64>,
}

impl RiskManager {
    pub fn new() -> Self {
        let max_opportunities_per_second = env::var("MAX_OPPORTUNITIES_PER_SECOND")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10);

        let max_size_per_opportunity = env::var("MAX_SIZE_PER_OPPORTUNITY")
            .unwrap_or_else(|_| "1000000000".to_string()) // 1 SOL in lamports
            .parse()
            .unwrap_or(1_000_000_000);

        let min_profit_threshold_bps = env::var("MIN_PROFIT_THRESHOLD_BPS")
            .unwrap_or_else(|_| "20".to_string()) // 0.2%
            .parse()
            .unwrap_or(20);

        let max_edge_age_ms = env::var("MAX_EDGE_AGE_MS")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .unwrap_or(50);

        let max_total_cu = env::var("MAX_TOTAL_CU")
            .unwrap_or_else(|_| "200000".to_string()) // 200k CU limit
            .parse()
            .unwrap_or(200_000);

        Self {
            max_opportunities_per_second,
            max_size_per_opportunity,
            min_profit_threshold_bps,
            max_edge_age_ms,
            max_total_cu,
            opportunities_this_second: Arc::new(AtomicU64::new(0)),
            last_reset_time: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            )),
        }
    }

    pub fn filter_opportunities(&self, opportunities: Vec<ArbitrageOpportunity>) -> Vec<ArbitrageOpportunity> {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Reset counter if we're in a new second
        let last_reset = self.last_reset_time.load(Ordering::Relaxed);
        if now_secs > last_reset {
            self.opportunities_this_second.store(0, Ordering::Relaxed);
            self.last_reset_time.store(now_secs, Ordering::Relaxed);
        }

        let mut filtered = Vec::new();
        let current_count = self.opportunities_this_second.load(Ordering::Relaxed);

        for opportunity in opportunities {
            // Check rate limiting
            if current_count + filtered.len() as u64 >= self.max_opportunities_per_second {
                break;
            }

            // Apply risk filters
            if !self.passes_risk_checks(&opportunity) {
                continue;
            }

            filtered.push(opportunity);
        }

        // Update counter
        self.opportunities_this_second.fetch_add(filtered.len() as u64, Ordering::Relaxed);

        filtered
    }

    fn passes_risk_checks(&self, opportunity: &ArbitrageOpportunity) -> bool {
        // Check minimum profit threshold
        if opportunity.profit_bps < self.min_profit_threshold_bps {
            return false;
        }

        // Check maximum size limit
        if opportunity.max_size_in > self.max_size_per_opportunity {
            return false;
        }

        // Check edge age (freshness)
        if opportunity.max_edge_age_ms > self.max_edge_age_ms {
            return false;
        }

        // Check compute unit limit
        if opportunity.total_cu_estimate > self.max_total_cu {
            return false;
        }

        // Check for reasonable number of hops (prevent complex routes)
        if opportunity.edges.len() > 3 {
            return false;
        }

        // Check that we have positive size
        if opportunity.max_size_in == 0 {
            return false;
        }

        // Additional safety checks
        if opportunity.total_fees_bps > 1000 { // More than 10% total fees is suspicious
            return false;
        }

        true
    }

    pub fn get_stats(&self) -> RiskStats {
        RiskStats {
            opportunities_this_second: self.opportunities_this_second.load(Ordering::Relaxed),
            max_opportunities_per_second: self.max_opportunities_per_second,
            max_size_per_opportunity: self.max_size_per_opportunity,
            min_profit_threshold_bps: self.min_profit_threshold_bps,
            max_edge_age_ms: self.max_edge_age_ms,
            max_total_cu: self.max_total_cu,
        }
    }
}

#[derive(Debug)]
pub struct RiskStats {
    pub opportunities_this_second: u64,
    pub max_opportunities_per_second: u64,
    pub max_size_per_opportunity: u128,
    pub min_profit_threshold_bps: u64,
    pub max_edge_age_ms: u64,
    pub max_total_cu: u32,
}

impl std::fmt::Display for RiskStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "RiskStats {{ opps_this_sec: {}/{}, max_size: {}, min_profit: {}bps, max_age: {}ms, max_cu: {} }}",
            self.opportunities_this_second,
            self.max_opportunities_per_second,
            self.max_size_per_opportunity,
            self.min_profit_threshold_bps,
            self.max_edge_age_ms,
            self.max_total_cu
        )
    }
}
