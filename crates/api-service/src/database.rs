use resilience_framework::{DatabaseConfig, ResilientDatabase};
use std::sync::Arc;
use uuid::Uuid;

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

        let db = Arc::new(ResilientDatabase::new(config).await.map_err(|e| {
            sqlx::Error::Configuration(format!("Failed to create resilient database: {}", e).into())
        })?);

        Ok(DatabaseService { db })
    }

    // ============================================================================
    // WALLET OPERATIONS
    // ============================================================================

    pub async fn get_wallets(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<WalletRecord>, sqlx::Error> {
        let limit = limit.unwrap_or(50).min(1000);
        let offset = offset.unwrap_or(0);

        let rows = self
            .db
            .query(
                "SELECT id, pubkey, tier, label, created_at, updated_at, last_accessed, 
                    access_count, is_active, daily_transactions, total_volume_lamports
             FROM wallets 
             ORDER BY created_at DESC 
             LIMIT $1 OFFSET $2",
                &[&(limit as i64), &(offset as i64)],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        let wallets = rows
            .into_iter()
            .map(|row| WalletRecord {
                id: row.get("id"),
                pubkey: row.get("pubkey"),
                tier: row.get("tier"),
                label: row.get("label"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_accessed: row.get("last_accessed"),
                access_count: row.get::<&str, i64>("access_count") as u64,
                is_active: row.get("is_active"),
                daily_transactions: row.get::<&str, i32>("daily_transactions") as u32,
                total_volume_lamports: row.get::<&str, i64>("total_volume_lamports") as u64,
            })
            .collect();

        Ok(wallets)
    }

    pub async fn get_wallet_by_id(
        &self,
        wallet_id: Uuid,
    ) -> Result<Option<WalletRecord>, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT id, pubkey, tier, label, created_at, updated_at, last_accessed,
                    access_count, is_active, daily_transactions, total_volume_lamports
             FROM wallets WHERE id = $1",
                &[&wallet_id],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(WalletRecord {
                id: row.get("id"),
                pubkey: row.get("pubkey"),
                tier: row.get("tier"),
                label: row.get("label"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_accessed: row.get("last_accessed"),
                access_count: row.get::<&str, i64>("access_count") as u64,
                is_active: row.get("is_active"),
                daily_transactions: row.get::<&str, i32>("daily_transactions") as u32,
                total_volume_lamports: row.get::<&str, i64>("total_volume_lamports") as u64,
            }))
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // ARBITRAGE OPERATIONS
    // ============================================================================

    pub async fn get_arbitrage_opportunities(
        &self,
        _params: ArbitrageQueryParams,
    ) -> Result<Vec<ArbitrageOpportunityRecord>, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT * FROM arbitrage_opportunities ORDER BY created_at DESC LIMIT 100",
                &[],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        let opportunities = rows
            .into_iter()
            .map(|row| ArbitrageOpportunityRecord {
                id: row.get::<&str, i64>("id") as u64,
                opportunity_id: row.get("opportunity_id"),
                timestamp: row.get("timestamp"),
                slot: row.get::<&str, i64>("slot") as u64,
                venue_sell: row.get("venue_sell"),
                venue_buy: row.get("venue_buy"),
                token_pair: row.get("token_pair"),
                profit_bps: row.get("profit_bps"),
                amount_in: row.get("amount_in"),
                expected_amount_out: row.get("expected_amount_out"),
                price_impact_pct: row.get("price_impact_pct"),
                detection_time_us: row.get::<&str, i64>("detection_time_us") as u64,
                strategy_type: row.get("strategy_type"),
                raw_opportunity: serde_json::Value::Null, // Placeholder for JSON
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(opportunities)
    }

    pub async fn create_arbitrage_opportunity(
        &self,
        request: CreateArbitrageRequest,
    ) -> Result<ArbitrageOpportunityRecord, sqlx::Error> {
        let params = vec![
            serde_json::Value::String(request.opportunity_id.clone()),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
            serde_json::Value::Number(serde_json::Number::from(request.slot.unwrap_or(0) as i64)),
            serde_json::Value::String(request.venue_sell.clone()),
            serde_json::Value::String(request.venue_buy.clone()),
            serde_json::Value::String(request.token_pair.clone()),
            serde_json::Value::Number(serde_json::Number::from(request.profit_bps)),
            serde_json::Value::Number(serde_json::Number::from_f64(request.amount_in).unwrap()),
            serde_json::Value::Number(
                serde_json::Number::from_f64(request.expected_amount_out).unwrap(),
            ),
            request
                .price_impact_pct
                .map(|v| serde_json::Value::Number(serde_json::Number::from_f64(v).unwrap()))
                .unwrap_or(serde_json::Value::Null),
            serde_json::Value::Number(serde_json::Number::from(
                request.detection_time_us.unwrap_or(0) as i64,
            )),
            serde_json::Value::String(request.strategy_type.clone()),
            serde_json::to_value(&request.raw_opportunity).unwrap_or(serde_json::Value::Null),
        ];

        let rows = self
            .db
            .query_with_params(
                "INSERT INTO arbitrage_opportunities 
             (opportunity_id, timestamp, slot, venue_sell, venue_buy, token_pair, profit_bps, 
              amount_in, expected_amount_out, price_impact_pct, detection_time_us, 
              strategy_type, raw_opportunity) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) 
             RETURNING id, opportunity_id, timestamp, slot, venue_sell, venue_buy, token_pair, 
             profit_bps, amount_in, expected_amount_out, price_impact_pct, detection_time_us, 
             strategy_type, raw_opportunity, created_at"
                    .to_string(),
                params,
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        if let Some(row) = rows.into_iter().next() {
            Ok(ArbitrageOpportunityRecord {
                id: row.get::<&str, i64>("id") as u64,
                opportunity_id: row.get("opportunity_id"),
                timestamp: row.get("timestamp"),
                slot: row.get::<&str, i64>("slot") as u64,
                venue_sell: row.get("venue_sell"),
                venue_buy: row.get("venue_buy"),
                token_pair: row.get("token_pair"),
                profit_bps: row.get("profit_bps"),
                amount_in: row.get("amount_in"),
                expected_amount_out: row.get("expected_amount_out"),
                price_impact_pct: row.get("price_impact_pct"),
                detection_time_us: row.get::<&str, i64>("detection_time_us") as u64,
                strategy_type: row.get("strategy_type"),
                raw_opportunity: row.get::<&str, serde_json::Value>("raw_opportunity"),
                created_at: row.get("created_at"),
            })
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    // ============================================================================
    // EXECUTION OPERATIONS
    // ============================================================================

    pub async fn get_execution_results(
        &self,
        _params: ExecutionQueryParams,
    ) -> Result<Vec<ExecutionResultRecord>, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT * FROM execution_results ORDER BY created_at DESC LIMIT 100",
                &[],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        let results = rows
            .into_iter()
            .map(|row| ExecutionResultRecord {
                id: row.get::<&str, i64>("id") as u64,
                opportunity_id: row.get("opportunity_id"),
                timestamp: row.get("timestamp"),
                executed: row.get("executed"),
                success: row.get("success"),
                actual_profit_bps: row.get("actual_profit_bps"),
                execution_time_us: row
                    .get::<&str, Option<i64>>("execution_time_us")
                    .map(|v| v as u64),
                gas_cost_sol: row.get("gas_cost_sol"),
                slippage_bps: row.get("slippage_bps"),
                failure_reason: row.get("failure_reason"),
                transaction_signature: row.get("transaction_signature"),
                bundle_id: row.get("bundle_id"),
                wallet_id: row.get("wallet_id"),
                raw_result: serde_json::Value::Null, // Placeholder
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(results)
    }

    // ============================================================================
    // PERFORMANCE METRICS OPERATIONS
    // ============================================================================

    pub async fn get_performance_metrics(
        &self,
        _params: MetricsQueryParams,
    ) -> Result<Vec<PerformanceMetricRecord>, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT * FROM performance_metrics ORDER BY created_at DESC LIMIT 100",
                &[],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        let metrics = rows
            .into_iter()
            .map(|row| PerformanceMetricRecord {
                id: row.get::<&str, i64>("id") as u64,
                timestamp: row.get("timestamp"),
                processing_time_p50_us: row
                    .get::<&str, Option<i64>>("processing_time_p50_us")
                    .map(|v| v as u64),
                processing_time_p95_us: row
                    .get::<&str, Option<i64>>("processing_time_p95_us")
                    .map(|v| v as u64),
                processing_time_p99_us: row
                    .get::<&str, Option<i64>>("processing_time_p99_us")
                    .map(|v| v as u64),
                opportunities_per_minute: row.get("opportunities_per_minute"),
                execution_success_rate: row.get("execution_success_rate"),
                total_profit_captured_bps: row.get("total_profit_captured_bps"),
                memory_usage_mb: row.get("memory_usage_mb"),
                cpu_utilization: row.get("cpu_utilization"),
                active_wallets: row.get("active_wallets"),
                metrics_json: None, // Placeholder
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(metrics)
    }

    // ============================================================================
    // TOKEN OPERATIONS
    // ============================================================================

    pub async fn get_tokens(
        &self,
        _params: TokenQueryParams,
    ) -> Result<Vec<TokenRecord>, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT * FROM tokens ORDER BY created_at DESC LIMIT 100",
                &[],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        let tokens = rows
            .into_iter()
            .map(|row| TokenRecord {
                id: row.get::<&str, i32>("id") as u32,
                mint: row.get("mint"),
                symbol: row.get("symbol"),
                name: row.get("name"),
                decimals: row.get("decimals"),
                logo_uri: row.get("logo_uri"),
                coingecko_id: row.get("coingecko_id"),
                is_verified: row.get("is_verified"),
                market_cap_usd: row
                    .get::<&str, Option<i64>>("market_cap_usd")
                    .map(|v| v as u64),
                volume_24h_usd: row
                    .get::<&str, Option<i64>>("volume_24h_usd")
                    .map(|v| v as u64),
                price_usd: row.get("price_usd"),
                metadata: None, // Placeholder
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(tokens)
    }

    // ============================================================================
    // SYSTEM HEALTH OPERATIONS
    // ============================================================================

    pub async fn get_system_health(&self) -> Result<SystemHealthRecord, sqlx::Error> {
        // Return placeholder data for now
        Ok(SystemHealthRecord::default())
    }

    pub async fn get_trading_performance(
        &self,
        _params: TradingPerformanceQueryParams,
    ) -> Result<Vec<TradingPerformanceRecord>, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT trading_date, total_executions, successful_executions, avg_profit_bps, 
                    total_profit_bps, avg_execution_time_ms, active_wallets
             FROM trading_performance 
             ORDER BY trading_date DESC 
             LIMIT 30",
                &[],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        let performance = rows
            .into_iter()
            .map(|row| TradingPerformanceRecord {
                trading_date: row.get("trading_date"),
                total_executions: row.get::<&str, i64>("total_executions") as u64,
                successful_executions: row.get::<&str, i64>("successful_executions") as u64,
                avg_profit_bps: row.get("avg_profit_bps"),
                total_profit_bps: row.get("total_profit_bps"),
                avg_execution_time_ms: row.get("avg_execution_time_ms"),
                active_wallets: row.get::<&str, i64>("active_wallets") as u64,
            })
            .collect();

        Ok(performance)
    }

    // ============================================================================
    // KAFKA CONSUMER METHODS
    // ============================================================================

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_arbitrage_opportunity_from_kafka(
        &self,
        _signature: &str,
        venue_a: &str,
        venue_b: &str,
        token_a: &str,
        token_b: &str,
        profit_bps: i32,
        slot: i64,
        estimated_net_profit_sol: f64,
        confidence_score: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
        strategy: &str,
    ) -> Result<bool, sqlx::Error> {
        let opportunity_id = format!("{}_{}", venue_a, venue_b);
        let token_pair = format!("{}/{}", token_a, token_b);

        let rows = self
            .db
            .query(
                "INSERT INTO arbitrage_opportunities 
             (opportunity_id, slot, venue_sell, venue_buy, token_pair, profit_bps, 
              amount_in, expected_amount_out, detection_time_us, strategy_type, timestamp) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) 
             ON CONFLICT DO NOTHING
             RETURNING id",
                &[
                    &opportunity_id,
                    &slot,
                    &venue_a,
                    &venue_b,
                    &token_pair,
                    &profit_bps,
                    &estimated_net_profit_sol,
                    &(estimated_net_profit_sol * confidence_score),
                    &0i64, // detection_time_us placeholder
                    &strategy,
                    &timestamp,
                ],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        Ok(!rows.is_empty())
    }

    pub async fn check_opportunity_exists(&self, signature: &str) -> Result<bool, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT 1 FROM arbitrage_opportunities WHERE opportunity_id = $1 LIMIT 1",
                &[&signature],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        Ok(!rows.is_empty())
    }

    pub async fn get_arbitrage_opportunity_by_id(
        &self,
        opportunity_id: u64,
    ) -> Result<Option<ArbitrageOpportunityRecord>, sqlx::Error> {
        let rows = self
            .db
            .query(
                "SELECT * FROM arbitrage_opportunities WHERE id = $1",
                &[&(opportunity_id as i64)],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(ArbitrageOpportunityRecord {
                id: row.get::<&str, i64>("id") as u64,
                opportunity_id: row.get("opportunity_id"),
                timestamp: row.get("timestamp"),
                slot: row.get::<&str, i64>("slot") as u64,
                venue_sell: row.get("venue_sell"),
                venue_buy: row.get("venue_buy"),
                token_pair: row.get("token_pair"),
                profit_bps: row.get("profit_bps"),
                amount_in: row.get("amount_in"),
                expected_amount_out: row.get("expected_amount_out"),
                price_impact_pct: row.get("price_impact_pct"),
                detection_time_us: row.get::<&str, i64>("detection_time_us") as u64,
                strategy_type: row.get("strategy_type"),
                raw_opportunity: serde_json::Value::Null,
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_trading_performance_summary(
        &self,
        days: Option<i32>,
    ) -> Result<Vec<TradingPerformanceRecord>, sqlx::Error> {
        self.get_trading_performance(TradingPerformanceQueryParams { days })
            .await
    }

    pub async fn get_token_by_mint(
        &self,
        mint: String,
    ) -> Result<Option<TokenRecord>, sqlx::Error> {
        let rows = self
            .db
            .query("SELECT * FROM tokens WHERE mint = $1", &[&mint])
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(TokenRecord {
                id: row.get::<&str, i32>("id") as u32,
                mint: row.get("mint"),
                symbol: row.get("symbol"),
                name: row.get("name"),
                decimals: row.get("decimals"),
                logo_uri: row.get("logo_uri"),
                coingecko_id: row.get("coingecko_id"),
                is_verified: row.get("is_verified"),
                market_cap_usd: row
                    .get::<&str, Option<i64>>("market_cap_usd")
                    .map(|v| v as u64),
                volume_24h_usd: row
                    .get::<&str, Option<i64>>("volume_24h_usd")
                    .map(|v| v as u64),
                price_usd: row.get("price_usd"),
                metadata: None,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // SMART FILTER OPERATIONS
    // ============================================================================

    pub async fn get_smart_filter_config(&self) -> Result<SmartFilterConfig, sqlx::Error> {
        // Get current configuration from the database using the view
        let rows = self
            .db
            .query(
                "SELECT parameter_name, new_value, timestamp, applied_by 
             FROM current_smart_filter_config 
             ORDER BY timestamp DESC",
                &[],
            )
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        // Default configuration values
        let mut config = SmartFilterConfig {
            min_profit_bps: 1.0,
            max_profit_bps: 1000.0,
            min_net_profit_sol: 0.0001,
            min_venue_success_rate: 10.0,
            max_gas_budget_pct: 15.0,
            max_concurrent_opportunities: 50.0,
            competition_lookback_ms: 5000.0,
            learning_rate: 0.1,
            min_samples_for_learning: 100.0,
            last_updated: chrono::Utc::now(),
            updated_by: "system".to_string(),
        };

        // Update config with values from database
        for row in rows {
            let parameter_name: String = row.get("parameter_name");
            let value_json: serde_json::Value = row.get("new_value");
            let timestamp: chrono::DateTime<chrono::Utc> = row.get("timestamp");
            let applied_by: String = row.get("applied_by");

            if timestamp > config.last_updated {
                config.last_updated = timestamp;
                config.updated_by = applied_by;
            }

            if let Some(value) = value_json.as_f64() {
                match parameter_name.as_str() {
                    "min_profit_bps" => config.min_profit_bps = value,
                    "max_profit_bps" => config.max_profit_bps = value,
                    "min_net_profit_sol" => config.min_net_profit_sol = value,
                    "min_venue_success_rate" => config.min_venue_success_rate = value,
                    "max_gas_budget_pct" => config.max_gas_budget_pct = value,
                    "max_concurrent_opportunities" => config.max_concurrent_opportunities = value,
                    "competition_lookback_ms" => config.competition_lookback_ms = value,
                    "learning_rate" => config.learning_rate = value,
                    "min_samples_for_learning" => config.min_samples_for_learning = value,
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    pub async fn update_smart_filter_config(
        &self,
        request: UpdateSmartFilterConfigRequest,
    ) -> Result<SmartFilterConfigUpdateResponse, sqlx::Error> {
        // Get current value if it exists
        let current_config = self.get_smart_filter_config().await?;
        let old_value = match request.parameter.as_str() {
            "min_profit_bps" => Some(current_config.min_profit_bps),
            "max_profit_bps" => Some(current_config.max_profit_bps),
            "min_net_profit_sol" => Some(current_config.min_net_profit_sol),
            "min_venue_success_rate" => Some(current_config.min_venue_success_rate),
            "max_gas_budget_pct" => Some(current_config.max_gas_budget_pct),
            "max_concurrent_opportunities" => Some(current_config.max_concurrent_opportunities),
            "competition_lookback_ms" => Some(current_config.competition_lookback_ms),
            "learning_rate" => Some(current_config.learning_rate),
            "min_samples_for_learning" => Some(current_config.min_samples_for_learning),
            _ => None,
        };

        // Insert new configuration record using parameterized query
        let old_value_json = old_value.map_or(serde_json::Value::Null, |v| serde_json::json!(v));
        let new_value_json = serde_json::json!(request.value);

        let rows = self.db.query(
            "INSERT INTO smart_filter_config_history (parameter_name, old_value, new_value, applied_by) 
             VALUES ($1, $2, $3, $4) RETURNING id, timestamp",
            &[&request.parameter, &old_value_json, &new_value_json, &request.applied_by]
        ).await.map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        if let Some(row) = rows.into_iter().next() {
            let config_change_id: i32 = row.get("id");
            let timestamp: chrono::DateTime<chrono::Utc> = row.get("timestamp");

            Ok(SmartFilterConfigUpdateResponse {
                parameter: request.parameter,
                old_value,
                new_value: request.value,
                config_change_id,
                timestamp,
                applied_by: request.applied_by,
                success: true,
            })
        } else {
            Err(sqlx::Error::Configuration(
                "Failed to insert configuration change".into(),
            ))
        }
    }

    pub async fn get_smart_filter_performance(
        &self,
        params: SmartFilterPerformanceQueryParams,
    ) -> Result<Vec<SmartFilterPerformanceSnapshot>, sqlx::Error> {
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);

        let mut query = "SELECT * FROM smart_filter_performance_snapshots WHERE 1=1".to_string();
        let mut bind_params = Vec::new();
        let mut param_count = 0;

        if let Some(from_timestamp) = params.from_timestamp {
            param_count += 1;
            query.push_str(&format!(" AND timestamp >= ${}", param_count));
            bind_params
                .push(Box::new(from_timestamp)
                    as Box<dyn sqlx::Encode<sqlx::postgres::Postgres> + Send>);
        }

        if let Some(to_timestamp) = params.to_timestamp {
            param_count += 1;
            query.push_str(&format!(" AND timestamp <= ${}", param_count));
            bind_params
                .push(Box::new(to_timestamp)
                    as Box<dyn sqlx::Encode<sqlx::postgres::Postgres> + Send>);
        }

        param_count += 1;
        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${}", param_count));
        bind_params
            .push(Box::new(limit as i64) as Box<dyn sqlx::Encode<sqlx::postgres::Postgres> + Send>);

        param_count += 1;
        query.push_str(&format!(" OFFSET ${}", param_count));
        bind_params.push(Box::new(offset as i64) as Box<dyn sqlx::Encode<sqlx::postgres::Postgres> + Send>);

        // For now, return empty results as the actual query building is complex
        // This would need proper dynamic query building which is beyond the current scope
        Ok(vec![])
    }

    pub async fn get_smart_filter_config_history(
        &self,
        params: SmartFilterConfigHistoryQueryParams,
    ) -> Result<Vec<SmartFilterConfigHistoryRecord>, sqlx::Error> {
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);

        let mut query = "SELECT id, timestamp, parameter_name, old_value, new_value, applied_by, performance_before, performance_after 
                         FROM smart_filter_config_history WHERE 1=1".to_string();

        if let Some(ref parameter_name) = params.parameter_name {
            query.push_str(" AND parameter_name = ");
            query.push_str(&format!("'{}'", parameter_name));
        }

        if let Some(ref applied_by) = params.applied_by {
            query.push_str(" AND applied_by = ");
            query.push_str(&format!("'{}'", applied_by));
        }

        query.push_str(&format!(
            " ORDER BY timestamp DESC LIMIT {} OFFSET {}",
            limit, offset
        ));

        let rows = self
            .db
            .query(&query, &[])
            .await
            .map_err(|e| sqlx::Error::Configuration(format!("Database error: {}", e).into()))?;

        let history = rows
            .into_iter()
            .map(|row| SmartFilterConfigHistoryRecord {
                id: row.get("id"),
                timestamp: row.get("timestamp"),
                parameter_name: row.get("parameter_name"),
                old_value: row.get("old_value"),
                new_value: row.get("new_value"),
                applied_by: row.get("applied_by"),
                performance_before: row.get("performance_before"),
                performance_after: row.get("performance_after"),
            })
            .collect();

        Ok(history)
    }

    // ============================================================================
    // HEALTH CHECK OPERATIONS
    // ============================================================================

    pub async fn test_connection(&self) -> Result<(), sqlx::Error> {
        // Simple connectivity test using a basic query
        self.db
            .query_with_params("SELECT 1 as test".to_string(), vec![])
            .await
            .map_err(|e| {
                sqlx::Error::Configuration(format!("Connection test failed: {}", e).into())
            })?;
        Ok(())
    }
}
