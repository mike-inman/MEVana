use crate::venue::VenueId;
use crate::graph::{Edge, EdgeGraph};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub opportunity_id: Uuid,
    pub token_in: Vec<u8>,
    pub token_out: Vec<u8>,
    pub edges: Vec<u64>, // Edge IDs forming the arbitrage path
    pub venues: Vec<VenueId>,
    pub profit_bps: u64,
    pub max_size_in: u128,
    pub total_cu_estimate: u32,
    pub total_fees_bps: u32,
    pub detected_at_ns: u64,
    pub max_edge_age_ms: u64,
}

#[derive(Clone)]
pub struct OpportunityDetector {
    min_profit_bps: u64,
    max_path_length: usize,
    max_opportunities_per_cycle: usize,
}

impl OpportunityDetector {
    pub fn new(min_profit_bps: u64) -> Self {
        Self {
            min_profit_bps,
            max_path_length: 3, // A->B->A or A->B->C->A
            max_opportunities_per_cycle: 50, // Limit to prevent overload
        }
    }

    pub async fn detect_opportunities(&self, graph: &EdgeGraph) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        let all_tokens = graph.get_all_tokens();
        
        // For performance, limit the number of tokens we check per cycle
        let tokens_to_check = all_tokens.iter().take(20).cloned().collect::<Vec<_>>();

        for token_start in &tokens_to_check {
            // Find 2-hop arbitrage: A -> B -> A
            if let Some(two_hop_opportunities) = self.find_two_hop_arbitrage(graph, token_start) {
                opportunities.extend(two_hop_opportunities);
            }

            // Find 3-hop arbitrage: A -> B -> C -> A
            if opportunities.len() < self.max_opportunities_per_cycle {
                if let Some(three_hop_opportunities) = self.find_three_hop_arbitrage(graph, token_start) {
                    opportunities.extend(three_hop_opportunities);
                }
            }

            // Limit total opportunities to prevent overload
            if opportunities.len() >= self.max_opportunities_per_cycle {
                break;
            }
        }

        // Sort by profit and return top opportunities
        opportunities.sort_by(|a, b| b.profit_bps.cmp(&a.profit_bps));
        opportunities.truncate(self.max_opportunities_per_cycle);
        
        opportunities
    }

    fn find_two_hop_arbitrage(&self, graph: &EdgeGraph, token_start: &[u8]) -> Option<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get all edges from token_start (A -> B)
        let from_start = graph.get_edges_from_token(token_start)?;
        
        for (intermediate_token, first_edges) in from_start {
            // Skip if intermediate token is the same as start
            if intermediate_token == token_start {
                continue;
            }
            
            // Get edges back to start token (B -> A)
            if let Some(to_start_edges) = graph.get_edges_to_token(token_start) {
                for (token, back_edges) in to_start_edges {
                    if token == intermediate_token {
                        // Try all combinations of first edge and back edge
                        for first_edge in &first_edges {
                            for back_edge in &back_edges {
                                if let Some(opportunity) = self.calculate_two_hop_profit(
                                    token_start,
                                    &intermediate_token,
                                    first_edge,
                                    back_edge
                                ) {
                                    opportunities.push(opportunity);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
        
        if opportunities.is_empty() {
            None
        } else {
            Some(opportunities)
        }
    }

    fn find_three_hop_arbitrage(&self, graph: &EdgeGraph, token_start: &[u8]) -> Option<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get all edges from token_start (A -> B)
        let from_start = graph.get_edges_from_token(token_start)?;
        
        for (token_b, first_edges) in from_start {
            if token_b == token_start {
                continue;
            }
            
            // Get edges from B to C
            if let Some(from_b) = graph.get_edges_from_token(&token_b) {
                for (token_c, second_edges) in from_b {
                    if token_c == token_start || token_c == token_b {
                        continue;
                    }
                    
                    // Get edges from C back to A
                    if let Some(to_start_edges) = graph.get_edges_to_token(token_start) {
                        for (token, back_edges) in to_start_edges {
                            if token == token_c {
                                // Try combinations (limited for performance)
                                for first_edge in first_edges.iter().take(3) {
                                    for second_edge in second_edges.iter().take(3) {
                                        for third_edge in back_edges.iter().take(3) {
                                            if let Some(opportunity) = self.calculate_three_hop_profit(
                                                token_start,
                                                &token_b,
                                                &token_c,
                                                first_edge,
                                                second_edge,
                                                third_edge
                                            ) {
                                                opportunities.push(opportunity);
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        if opportunities.is_empty() {
            None
        } else {
            Some(opportunities)
        }
    }

    fn calculate_two_hop_profit(
        &self,
        token_start: &[u8],
        token_intermediate: &[u8],
        first_edge: &Edge,
        back_edge: &Edge,
    ) -> Option<ArbitrageOpportunity> {
        // Simple marginal price calculation for quick filtering
        // In practice, you'd use the full quoter for accurate calculations
        
        // Calculate effective exchange rate: start -> intermediate -> start
        let rate_forward = first_edge.marginal_price_q64;
        let rate_back = back_edge.marginal_price_q64;
        
        if rate_forward == 0 || rate_back == 0 {
            return None;
        }
        
        // Calculate round-trip efficiency (simplified)
        // This is a rough approximation - real implementation would use quoter
        let q64_one = 1u128 << 64;
        let net_rate = (rate_forward * rate_back) / q64_one;
        
        // Account for fees
        let total_fees_bps = first_edge.fee_bps_total + back_edge.fee_bps_total;
        let fee_multiplier = (10000 - total_fees_bps as u128) * (10000 - total_fees_bps as u128) / (10000 * 10000);
        let net_rate_after_fees = (net_rate * fee_multiplier) / q64_one;
        
        // Calculate profit in basis points
        if net_rate_after_fees <= q64_one {
            return None; // No profit
        }
        
        let profit_ratio = net_rate_after_fees - q64_one;
        let profit_bps = (profit_ratio * 10000) / q64_one;
        
        if profit_bps < self.min_profit_bps as u128 {
            return None;
        }
        
        // Calculate max size based on liquidity constraints
        let max_size_in = first_edge.size_to_next_boundary_in.min(back_edge.size_to_next_boundary_in);
        
        if max_size_in == 0 {
            return None;
        }
        
        // Calculate edge ages
        let max_edge_age_ms = first_edge.age_ms().max(back_edge.age_ms());
        
        Some(ArbitrageOpportunity {
            opportunity_id: Uuid::new_v4(),
            token_in: token_start.to_vec(),
            token_out: token_start.to_vec(), // Round trip
            edges: vec![first_edge.edge_id, back_edge.edge_id],
            venues: vec![first_edge.venue_id, back_edge.venue_id],
            profit_bps: profit_bps as u64,
            max_size_in,
            total_cu_estimate: first_edge.cu_estimate + back_edge.cu_estimate,
            total_fees_bps,
            detected_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            max_edge_age_ms,
        })
    }

    fn calculate_three_hop_profit(
        &self,
        token_start: &[u8],
        _token_b: &[u8],
        _token_c: &[u8],
        first_edge: &Edge,
        second_edge: &Edge,
        third_edge: &Edge,
    ) -> Option<ArbitrageOpportunity> {
        // Similar to two-hop but with three legs
        let rate_1 = first_edge.marginal_price_q64;
        let rate_2 = second_edge.marginal_price_q64;
        let rate_3 = third_edge.marginal_price_q64;
        
        if rate_1 == 0 || rate_2 == 0 || rate_3 == 0 {
            return None;
        }
        
        let q64_one = 1u128 << 64;
        
        // Calculate compound rate (simplified)
        let compound_rate = ((rate_1 * rate_2) / q64_one * rate_3) / q64_one;
        
        // Account for fees
        let total_fees_bps = first_edge.fee_bps_total + second_edge.fee_bps_total + third_edge.fee_bps_total;
        let fee_factor = 10000u32.saturating_sub(total_fees_bps);
        let fee_multiplier = (fee_factor as u128).pow(3) / (10000u128.pow(3));
        let net_rate = (compound_rate * fee_multiplier) / q64_one;
        
        if net_rate <= q64_one {
            return None;
        }
        
        let profit_ratio = net_rate - q64_one;
        let profit_bps = (profit_ratio * 10000) / q64_one;
        
        if profit_bps < self.min_profit_bps as u128 {
            return None;
        }
        
        // Size limited by smallest edge
        let max_size_in = first_edge.size_to_next_boundary_in
            .min(second_edge.size_to_next_boundary_in)
            .min(third_edge.size_to_next_boundary_in);
        
        if max_size_in == 0 {
            return None;
        }
        
        let max_edge_age_ms = first_edge.age_ms()
            .max(second_edge.age_ms())
            .max(third_edge.age_ms());
        
        Some(ArbitrageOpportunity {
            opportunity_id: Uuid::new_v4(),
            token_in: token_start.to_vec(),
            token_out: token_start.to_vec(),
            edges: vec![first_edge.edge_id, second_edge.edge_id, third_edge.edge_id],
            venues: vec![first_edge.venue_id, second_edge.venue_id, third_edge.venue_id],
            profit_bps: profit_bps as u64,
            max_size_in,
            total_cu_estimate: first_edge.cu_estimate + second_edge.cu_estimate + third_edge.cu_estimate,
            total_fees_bps,
            detected_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            max_edge_age_ms,
        })
    }
}