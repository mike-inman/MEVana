use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Pair {
    pub a: String,
    pub b: String,
}
impl Pair {
    pub fn canonical(a: &str, b: &str) -> Self {
        if a <= b {
            Self {
                a: a.into(),
                b: b.into(),
            }
        } else {
            Self {
                a: b.into(),
                b: a.into(),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbSignalV1 {
    pub pair: Pair,
    pub dex_sell: String,
    pub dex_mid: Option<String>,
    pub dex_buy: String,
    pub amount_in: u128,
    pub expected_profit_out: i128,
    pub slot: u64,
    pub sell_pool_id: String,
    pub mid_pool_id: Option<String>,
    pub buy_pool_id: String,
    pub id: u64,
    pub profit_bps: i32,
    pub emitted_at_ns: u64,
}

// Enhanced arbitrage signal for Jupiter multi-hop routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterArbSignalV1 {
    pub opportunity_id: String,
    pub route_type: RouteType,
    pub input_token: String,
    pub output_token: String,
    pub amount_in: u128,
    pub expected_amount_out: u128,
    pub minimum_amount_out: u128,
    pub profit_bps: i32,
    pub route_hops: Vec<RouteHop>,
    pub total_price_impact: f64,
    pub estimated_fees: u64,
    pub max_slippage_bps: u16,
    pub deadline_slot: u64,
    pub flash_loan_required: bool,
    pub priority_fee_lamports: u64,
    pub jupiter_route_plan: String, // JSON serialized Jupiter route
    pub emitted_at_ns: u64,
    pub expires_at_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHop {
    pub from_token: String,
    pub to_token: String,
    pub venue: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
    pub price_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteType {
    DirectArbitrage,     // Traditional A->B->A
    TwoHopArbitrage,     // A->B->A via Jupiter
    ThreeHopArbitrage,   // A->B->C->A via Jupiter
    FourHopArbitrage,    // A->B->C->D->A via Jupiter
    CrossDexArbitrage,   // Multiple DEX arbitrage
    TriangularArbitrage, // A->B->C->A on same DEX
}

// Enhanced arbitrage signal with both traditional and Jupiter support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedArbSignal {
    pub traditional: Option<ArbSignalV1>,
    pub jupiter: Option<JupiterArbSignalV1>,
    pub signal_type: ArbSignalType,
    pub confidence_score: f64, // 0.0-1.0 confidence in opportunity
    pub paper_trading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArbSignalType {
    Traditional,
    Jupiter,
    Hybrid, // Uses both traditional and Jupiter routing
}

// Keep enum variants stack-allocated for hotpath performance
// Allow large enum variant for ultra-low-latency trading
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "v", content = "data")]
pub enum ArbSignal {
    V1(ArbSignalV1),
    Jupiter(JupiterArbSignalV1),
    Enhanced(EnhancedArbSignal),
}

impl ArbSignal {
    pub fn v1(s: ArbSignalV1) -> Self {
        Self::V1(s)
    }

    pub fn jupiter(s: JupiterArbSignalV1) -> Self {
        Self::Jupiter(s)
    }

    pub fn enhanced(s: EnhancedArbSignal) -> Self {
        Self::Enhanced(s)
    }

    pub fn get_profit_bps(&self) -> i32 {
        match self {
            ArbSignal::V1(s) => s.profit_bps,
            ArbSignal::Jupiter(s) => s.profit_bps,
            ArbSignal::Enhanced(s) => s
                .jupiter
                .as_ref()
                .map(|j| j.profit_bps)
                .or_else(|| s.traditional.as_ref().map(|t| t.profit_bps))
                .unwrap_or(0),
        }
    }

    pub fn is_paper_trading(&self) -> bool {
        match self {
            ArbSignal::V1(_) => false,      // Traditional signals are live by default
            ArbSignal::Jupiter(_) => false, // Jupiter signals are live by default
            ArbSignal::Enhanced(s) => s.paper_trading,
        }
    }

    pub fn get_amount_in(&self) -> u128 {
        match self {
            ArbSignal::V1(s) => s.amount_in,
            ArbSignal::Jupiter(s) => s.amount_in,
            ArbSignal::Enhanced(s) => s
                .jupiter
                .as_ref()
                .map(|j| j.amount_in)
                .or_else(|| s.traditional.as_ref().map(|t| t.amount_in))
                .unwrap_or(0),
        }
    }
}
