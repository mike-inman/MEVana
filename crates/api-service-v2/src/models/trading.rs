//! Trading-related models

use crate::database::Entity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Trade entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Trade {
    pub id: Uuid,
    pub user_id: Uuid,
    pub wallet_id: Uuid,
    pub trade_type: TradeType,
    pub strategy: TradingStrategy,
    pub input_token: String,
    pub output_token: String,
    pub input_amount: f64,
    pub output_amount: Option<f64>,
    pub expected_output: f64,
    pub slippage_tolerance: f64,
    pub status: TradeStatus,
    pub signature: Option<String>,
    pub profit_loss: Option<f64>,
    pub fees: f64,
    pub executed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Entity for Trade {
    type Id = Uuid;
    
    fn id(&self) -> &Self::Id {
        &self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = id;
    }
}

/// Trade type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TradeType {
    Manual,
    Automated,
    Arbitrage,
}

/// Trading strategy enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TradingStrategy {
    Simple,
    Arbitrage,
    DollarCostAverage,
    GridTrading,
    Custom,
}

/// Trade status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TradeStatus {
    Pending,
    Queued,
    Executing,
    Completed,
    Failed,
    Cancelled,
    PartiallyFilled,
}

/// Trade request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct TradeRequest {
    pub wallet_id: Uuid,
    
    pub input_token: String,
    
    pub output_token: String,
    
    #[validate(range(min = 0.0))]
    pub input_amount: f64,
    
    #[validate(range(min = 0.0, max = 100.0))]
    pub slippage_tolerance: f64,
    
    pub strategy: Option<TradingStrategy>,
}

/// Trade response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TradeResponse {
    pub id: Uuid,
    pub trade_type: TradeType,
    pub strategy: TradingStrategy,
    pub input_token: String,
    pub output_token: String,
    pub input_amount: f64,
    pub output_amount: Option<f64>,
    pub expected_output: f64,
    pub slippage_tolerance: f64,
    pub status: TradeStatus,
    pub signature: Option<String>,
    pub profit_loss: Option<f64>,
    pub fees: f64,
    pub executed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<Trade> for TradeResponse {
    fn from(trade: Trade) -> Self {
        Self {
            id: trade.id,
            trade_type: trade.trade_type,
            strategy: trade.strategy,
            input_token: trade.input_token,
            output_token: trade.output_token,
            input_amount: trade.input_amount,
            output_amount: trade.output_amount,
            expected_output: trade.expected_output,
            slippage_tolerance: trade.slippage_tolerance,
            status: trade.status,
            signature: trade.signature,
            profit_loss: trade.profit_loss,
            fees: trade.fees,
            executed_at: trade.executed_at,
            created_at: trade.created_at,
        }
    }
}

/// Trade history query parameters
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::IntoParams)]
pub struct TradeHistoryQuery {
    pub wallet_id: Option<Uuid>,
    pub trade_type: Option<TradeType>,
    pub strategy: Option<TradingStrategy>,
    pub status: Option<TradeStatus>,
    pub from_date: Option<chrono::DateTime<chrono::Utc>>,
    pub to_date: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Trade history response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TradeHistory {
    pub trades: Vec<TradeResponse>,
    pub summary: TradeHistorySummary,
}

/// Trade history summary
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TradeHistorySummary {
    pub total_trades: i64,
    pub successful_trades: i64,
    pub failed_trades: i64,
    pub total_volume: f64,
    pub total_profit_loss: f64,
    pub total_fees: f64,
    pub success_rate: f64,
    pub average_profit: f64,
}

/// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArbitrageOpportunity {
    pub id: Uuid,
    pub input_token: String,
    pub output_token: String,
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub profit_percentage: f64,
    pub estimated_profit: f64,
    pub max_amount: f64,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Trading strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StrategyConfig {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub strategy: TradingStrategy,
    pub parameters: serde_json::Value,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}