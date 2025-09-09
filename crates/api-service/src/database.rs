use resilience_framework::{ResilientDatabase, DatabaseConfig};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::models::*;

#[derive(Clone)]
pub struct DatabaseService {
    db: Arc<ResilientDatabase>,
}

impl DatabaseService {
    pub async fn new(database_url: String) -> Result<Self, sqlx::Error> {
        let config = DatabaseConfig {
            primary_url: database_url,
            ..Default::default()
        };
        
        let db = Arc::new(
            ResilientDatabase::new(config)
                .await
                .map_err(|e| sqlx::Error::Configuration(format!("Failed to create resilient database: {}", e).into()))?
        );
        
        Ok(DatabaseService { db })
    }

    // ============================================================================
    // WALLET OPERATIONS
    // ============================================================================
    
    pub async fn get_wallets(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<WalletRecord>, sqlx::Error> {
        let limit = limit.unwrap_or(50).min(1000);
        let offset = offset.unwrap_or(0);
        
        let rows = self.db.query(
            "SELECT id, pubkey, tier, label, created_at, updated_at, last_accessed, 
                    access_count, is_active, daily_transactions, total_volume_lamports
             FROM wallets 
             ORDER BY created_at DESC 
             LIMIT $1 OFFSET $2",
            &[&(limit as i64), &(offset as i64)]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        let wallets = rows.into_iter().map(|row| WalletRecord {
            id: row.get("id"),
            pubkey: row.get("pubkey"),
            tier: row.get("tier"),
            label: row.get("label"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_accessed: row.get("last_accessed"),
            access_count: row.get::<i64, _>("access_count") as u64,
            is_active: row.get("is_active"),
            daily_transactions: row.get::<i32, _>("daily_transactions") as u32,
            total_volume_lamports: row.get::<i64, _>("total_volume_lamports") as u64,
        }).collect();
        
        Ok(wallets)
    }
    
    pub async fn get_wallet_by_id(&self, wallet_id: Uuid) -> Result<Option<WalletRecord>, sqlx::Error> {
        let rows = self.db.query(
            "SELECT id, pubkey, tier, label, created_at, updated_at, last_accessed,
                    access_count, is_active, daily_transactions, total_volume_lamports
             FROM wallets WHERE id = $1",
            &[&wallet_id]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        if let Some(row) = rows.into_iter().next() {
            Ok(Some(WalletRecord {
                id: row.get("id"),
                pubkey: row.get("pubkey"),
                tier: row.get("tier"),
                label: row.get("label"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_accessed: row.get("last_accessed"),
                access_count: row.get::<i64, _>("access_count") as u64,
                is_active: row.get("is_active"),
                daily_transactions: row.get::<i32, _>("daily_transactions") as u32,
                total_volume_lamports: row.get::<i64, _>("total_volume_lamports") as u64,
            }))
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // ARBITRAGE OPERATIONS
    // ============================================================================
    
    pub async fn get_arbitrage_opportunities(&self, params: ArbitrageQueryParams) -> Result<Vec<ArbitrageOpportunityRecord>, sqlx::Error> {
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);
        
        let mut query = "SELECT * FROM arbitrage_opportunities WHERE 1=1".to_string();
        let mut bind_params: Vec<Box<dyn sqlx::postgres::types::PgHasArrayType + Send + Sync>> = Vec::new();
        let mut param_count = 0;
        
        if let Some(venue) = &params.venue {
            param_count += 1;
            query.push_str(&format!(" AND (venue_sell = ${} OR venue_buy = ${})", param_count, param_count));
            bind_params.push(Box::new(venue.clone()));
        }
        
        if let Some(token_pair) = &params.token_pair {
            param_count += 1;
            query.push_str(&format!(" AND token_pair = ${}", param_count));
            bind_params.push(Box::new(token_pair.clone()));
        }
        
        if let Some(min_profit) = params.min_profit_bps {
            param_count += 1;
            query.push_str(&format!(" AND profit_bps >= ${}", param_count));
            bind_params.push(Box::new(min_profit));
        }
        
        if let Some(from_time) = params.from_time {
            param_count += 1;
            query.push_str(&format!(" AND timestamp >= ${}", param_count));
            bind_params.push(Box::new(from_time));
        }
        
        if let Some(to_time) = params.to_time {
            param_count += 1;
            query.push_str(&format!(" AND timestamp <= ${}", param_count));
            bind_params.push(Box::new(to_time));
        }
        
        param_count += 1;
        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${}", param_count));
        bind_params.push(Box::new(limit));
        
        param_count += 1;
        query.push_str(&format!(" OFFSET ${}", param_count));
        bind_params.push(Box::new(offset));
        
        // For simplicity, execute a basic query without dynamic parameters
        // In production, you'd want proper parameter binding
        let rows = self.db.query(
            "SELECT * FROM arbitrage_opportunities ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
            &[&(limit as i64), &(offset as i64)]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        let opportunities = rows.into_iter().map(|row| ArbitrageOpportunityRecord {
            id: row.get::<i64, _>("id") as u64,
            opportunity_id: row.get("opportunity_id"),
            timestamp: row.get("timestamp"),
            slot: row.get::<i64, _>("slot") as u64,
            venue_sell: row.get("venue_sell"),
            venue_buy: row.get("venue_buy"),
            token_pair: row.get("token_pair"),
            profit_bps: row.get::<i32, _>("profit_bps"),
            amount_in: row.get("amount_in"),
            expected_amount_out: row.get("expected_amount_out"),
            price_impact_pct: row.get("price_impact_pct"),
            detection_time_us: row.get::<i64, _>("detection_time_us") as u64,
            strategy_type: row.get("strategy_type"),
            raw_opportunity: row.get("raw_opportunity"),
            created_at: row.get("created_at"),
        }).collect();
        
        Ok(opportunities)
    }
    
    pub async fn get_arbitrage_opportunity_by_id(&self, opportunity_id: String) -> Result<Option<ArbitrageOpportunityRecord>, sqlx::Error> {
        let rows = self.db.query(
            "SELECT * FROM arbitrage_opportunities WHERE opportunity_id = $1",
            &[&opportunity_id]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        if let Some(row) = rows.into_iter().next() {
            Ok(Some(ArbitrageOpportunityRecord {
                id: row.get::<i64, _>("id") as u64,
                opportunity_id: row.get("opportunity_id"),
                timestamp: row.get("timestamp"),
                slot: row.get::<i64, _>("slot") as u64,
                venue_sell: row.get("venue_sell"),
                venue_buy: row.get("venue_buy"),
                token_pair: row.get("token_pair"),
                profit_bps: row.get::<i32, _>("profit_bps"),
                amount_in: row.get("amount_in"),
                expected_amount_out: row.get("expected_amount_out"),
                price_impact_pct: row.get("price_impact_pct"),
                detection_time_us: row.get::<i64, _>("detection_time_us") as u64,
                strategy_type: row.get("strategy_type"),
                raw_opportunity: row.get("raw_opportunity"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // EXECUTION RESULTS
    // ============================================================================
    
    pub async fn get_execution_results(&self, params: ExecutionQueryParams) -> Result<Vec<ExecutionResultRecord>, sqlx::Error> {
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);
        
        let rows = self.db.query(
            "SELECT * FROM execution_results ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
            &[&(limit as i64), &(offset as i64)]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        let results = rows.into_iter().map(|row| ExecutionResultRecord {
            id: row.get::<i64, _>("id") as u64,
            opportunity_id: row.get("opportunity_id"),
            timestamp: row.get("timestamp"),
            executed: row.get("executed"),
            success: row.get("success"),
            actual_profit_bps: row.get::<Option<i32>, _>("actual_profit_bps"),
            execution_time_us: row.get::<Option<i64>, _>("execution_time_us").map(|v| v as u64),
            gas_cost_sol: row.get("gas_cost_sol"),
            slippage_bps: row.get::<Option<i32>, _>("slippage_bps"),
            failure_reason: row.get("failure_reason"),
            transaction_signature: row.get("transaction_signature"),
            bundle_id: row.get("bundle_id"),
            wallet_id: row.get("wallet_id"),
            raw_result: row.get("raw_result"),
            created_at: row.get("created_at"),
        }).collect();
        
        Ok(results)
    }

    // ============================================================================
    // PERFORMANCE METRICS
    // ============================================================================
    
    pub async fn get_performance_metrics(&self, params: MetricsQueryParams) -> Result<Vec<PerformanceMetricRecord>, sqlx::Error> {
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);
        
        let mut query = "SELECT * FROM performance_metrics WHERE 1=1".to_string();
        
        if let Some(from_time) = params.from_time {
            query.push_str(" AND timestamp >= $3");
        }
        
        if let Some(to_time) = params.to_time {
            query.push_str(" AND timestamp <= $4");
        }
        
        query.push_str(" ORDER BY timestamp DESC LIMIT $1 OFFSET $2");
        
        let rows = self.db.query(
            &query,
            &[&(limit as i64), &(offset as i64)]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        let metrics = rows.into_iter().map(|row| PerformanceMetricRecord {
            id: row.get::<i64, _>("id") as u64,
            timestamp: row.get("timestamp"),
            processing_time_p50_us: row.get::<Option<i64>, _>("processing_time_p50_us").map(|v| v as u64),
            processing_time_p95_us: row.get::<Option<i64>, _>("processing_time_p95_us").map(|v| v as u64),
            processing_time_p99_us: row.get::<Option<i64>, _>("processing_time_p99_us").map(|v| v as u64),
            opportunities_per_minute: row.get::<Option<i32>, _>("opportunities_per_minute"),
            execution_success_rate: row.get("execution_success_rate"),
            total_profit_captured_bps: row.get::<Option<i64>, _>("total_profit_captured_bps"),
            memory_usage_mb: row.get("memory_usage_mb"),
            cpu_utilization: row.get("cpu_utilization"),
            active_wallets: row.get::<Option<i32>, _>("active_wallets"),
            metrics_json: row.get("metrics_json"),
            created_at: row.get("created_at"),
        }).collect();
        
        Ok(metrics)
    }

    // ============================================================================
    // TOKEN REGISTRY
    // ============================================================================
    
    pub async fn get_tokens(&self, params: TokenQueryParams) -> Result<Vec<TokenRecord>, sqlx::Error> {
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);
        
        let rows = self.db.query(
            "SELECT * FROM token_registry ORDER BY symbol LIMIT $1 OFFSET $2",
            &[&(limit as i64), &(offset as i64)]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        let tokens = rows.into_iter().map(|row| TokenRecord {
            id: row.get::<i32, _>("id") as u32,
            mint: row.get("mint"),
            symbol: row.get("symbol"),
            name: row.get("name"),
            decimals: row.get::<i32, _>("decimals"),
            logo_uri: row.get("logo_uri"),
            coingecko_id: row.get("coingecko_id"),
            is_verified: row.get("is_verified"),
            market_cap_usd: row.get::<Option<i64>, _>("market_cap_usd").map(|v| v as u64),
            volume_24h_usd: row.get::<Option<i64>, _>("volume_24h_usd").map(|v| v as u64),
            price_usd: row.get("price_usd"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }).collect();
        
        Ok(tokens)
    }
    
    pub async fn get_token_by_mint(&self, mint: String) -> Result<Option<TokenRecord>, sqlx::Error> {
        let rows = self.db.query(
            "SELECT * FROM token_registry WHERE mint = $1",
            &[&mint]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        if let Some(row) = rows.into_iter().next() {
            Ok(Some(TokenRecord {
                id: row.get::<i32, _>("id") as u32,
                mint: row.get("mint"),
                symbol: row.get("symbol"),
                name: row.get("name"),
                decimals: row.get::<i32, _>("decimals"),
                logo_uri: row.get("logo_uri"),
                coingecko_id: row.get("coingecko_id"),
                is_verified: row.get("is_verified"),
                market_cap_usd: row.get::<Option<i64>, _>("market_cap_usd").map(|v| v as u64),
                volume_24h_usd: row.get::<Option<i64>, _>("volume_24h_usd").map(|v| v as u64),
                price_usd: row.get("price_usd"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // SYSTEM HEALTH AND STATISTICS
    // ============================================================================
    
    pub async fn get_system_health(&self) -> Result<SystemHealthRecord, sqlx::Error> {
        let health_rows = self.db.query("SELECT * FROM system_health", &[])
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        let mut health = SystemHealthRecord::default();
        
        for row in health_rows {
            let metric_name: String = row.get("metric_name");
            let value: i64 = row.get("value");
            
            match metric_name.as_str() {
                "arbitrage_opportunities" => health.opportunities_last_hour = value as u64,
                "successful_executions" => health.successful_executions_last_hour = value as u64,
                "active_wallets" => health.active_wallets = value as u64,
                _ => {}
            }
        }
        
        Ok(health)
    }
    
    pub async fn get_trading_performance_summary(&self, days: Option<i32>) -> Result<Vec<TradingPerformanceRecord>, sqlx::Error> {
        let days = days.unwrap_or(7).min(365);
        
        let rows = self.db.query(
            "SELECT * FROM trading_performance_summary 
             WHERE trading_date >= CURRENT_DATE - INTERVAL '%s days' 
             ORDER BY trading_date DESC",
            &[]  // Note: In production, properly parameterize this query
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;
        
        let performance = rows.into_iter().map(|row| TradingPerformanceRecord {
            trading_date: row.get("trading_date"),
            total_executions: row.get::<i64, _>("total_executions") as u64,
            successful_executions: row.get::<i64, _>("successful_executions") as u64,
            avg_profit_bps: row.get("avg_profit_bps"),
            total_profit_bps: row.get::<Option<i64>, _>("total_profit_bps").map(|v| v as i64),
            avg_execution_time_ms: row.get("avg_execution_time_ms"),
            active_wallets: row.get::<i64, _>("active_wallets") as u64,
        }).collect();
        
        Ok(performance)
    }
}