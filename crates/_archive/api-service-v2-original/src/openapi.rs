//! OpenAPI documentation generation
//!
//! Comprehensive Swagger/OpenAPI documentation for the MEVana API v2

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};

use crate::{
    handlers::{arbitrage, auth, health, trading, wallets},
    models::{arbitrage::*, wallets::*, trading::*},
};

/// Main OpenAPI specification
#[derive(OpenApi)]
#[openapi(
    info(
        title = "MEVana API v2",
        version = "2.0.0",
        description = "Enterprise-grade API for MEV arbitrage trading on Solana",
        contact(
            name = "MEVana Team",
            url = "https://mevana.ai",
            email = "support@mevana.ai"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:3000", description = "Development server"),
        (url = "https://api.mevana.ai", description = "Production server"),
        (url = "https://staging-api.mevana.ai", description = "Staging server")
    ),
    paths(
        // Health endpoints
        health::health_check,
        
        // Authentication endpoints
        auth::login,
        auth::refresh_token,
        auth::logout,
        auth::register,
        auth::change_password,
        auth::verify_email,
        auth::reset_password,
        
        // Arbitrage endpoints
        arbitrage::create_arbitrage_opportunity,
        arbitrage::get_arbitrage_opportunities,
        arbitrage::get_arbitrage_opportunity_by_id,
        arbitrage::get_execution_results,
        arbitrage::update_opportunity_status,
        arbitrage::get_arbitrage_analytics,
        arbitrage::get_performance_metrics,
        arbitrage::get_real_time_opportunities,
        arbitrage::create_custom_strategy,
        arbitrage::get_strategy_performance,
        
        // Wallet management endpoints
        wallets::create_wallet,
        wallets::get_wallets,
        wallets::get_wallet_by_id,
        wallets::update_wallet,
        wallets::get_wallet_analytics,
        wallets::get_wallet_transactions,
        wallets::audit_wallet_security,
        wallets::emergency_lock_wallet,
        wallets::get_wallet_portfolio,
        wallets::get_wallet_risk_assessment,
        wallets::bulk_wallet_operations,
        
        // Trading endpoints
        trading::get_trading_dashboard,
        trading::get_market_data,
        trading::get_venue_status,
        trading::get_liquidity_analysis,
        trading::execute_trade,
        trading::get_trade_history,
        trading::get_pnl_report,
        trading::get_risk_metrics,
        trading::update_trading_parameters,
        trading::emergency_stop_trading,
        
        // Analytics endpoints
        analytics::get_system_overview,
        analytics::get_venue_performance,
        analytics::get_token_analysis,
        analytics::get_profit_attribution,
        analytics::get_market_conditions,
        analytics::get_competition_analysis,
        analytics::export_data,
        
        // Real-time streaming endpoints
        streaming::subscribe_opportunities,
        streaming::subscribe_executions,
        streaming::subscribe_market_data,
        streaming::subscribe_system_alerts,
        
        // Administrative endpoints
        admin::get_system_health,
        admin::get_audit_logs,
        admin::manage_users,
        admin::system_configuration,
        admin::emergency_controls,
    ),
    components(
        schemas(
            // Core models
            ArbitrageOpportunityRecord,
            ExecutionResultRecord,
            ArbitragePerformanceRecord,
            CreateArbitrageOpportunityRequest,
            ArbitrageOpportunityResponse,
            UpdateOpportunityStatusRequest,
            ArbitrageQueryParams,
            ExecutionQueryParams,
            PerformanceQueryParams,
            OpportunityStatus,
            ExecutionMethod,
            RiskLevel,
            ArbitrageAnalytics,
            VenuePerformance,
            StrategyPerformance,
            MarketConditions,
            RiskMetrics,
            PaginatedResponse<ArbitrageOpportunityRecord>,
            PaginationInfo,
            
            // Wallet models
            WalletRecord,
            TransactionRecord,
            WalletAnalytics,
            WalletPortfolio,
            TokenHolding,
            PositionInfo,
            CreateWalletRequest,
            UpdateWalletRequest,
            EmergencyLockRequest,
            WalletResponse,
            WalletDetailResponse,
            WalletBalance,
            SecurityStatus,
            ComplianceInfo,
            ActivitySummary,
            TransactionSummary,
            WalletQueryParams,
            WalletAnalyticsQuery,
            TransactionQueryParams,
            WalletRiskAssessment,
            RiskFactor,
            SecurityAuditReport,
            SecurityCheck,
            SecurityVulnerability,
            SecurityRecommendation,
            BulkWalletOperationRequest,
            BulkOperationResponse,
            BulkOperationResult,
            WalletType,
            SecurityTier,
            WalletStatus,
            ComplianceStatus,
            TransactionType,
            TransactionStatus,
            BulkOperationType,
            
            // Trading models
            TradingDashboard,
            MarketData,
            VenueStatus,
            LiquidityAnalysis,
            TradeRequest,
            TradeResponse,
            TradeHistory,
            PnLReport,
            TradingRiskMetrics,
            TradingParameters,
            
            // Authentication models
            LoginRequest,
            LoginResponse,
            RefreshTokenRequest,
            RegisterRequest,
            ChangePasswordRequest,
            UserInfo,
            
            // Error models
            ApiError,
            ErrorDetails,
            ValidationError,
            
            // Real-time models
            StreamSubscription,
            StreamMessage,
            AlertSeverity,
            SystemAlert,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication and authorization"),
        (name = "arbitrage", description = "Arbitrage opportunity management"),
        (name = "wallets", description = "Enterprise wallet management"),
        (name = "trading", description = "Trading operations and analytics"),
        (name = "analytics", description = "Advanced analytics and reporting"),
        (name = "streaming", description = "Real-time data streaming"),
        (name = "admin", description = "Administrative operations")
    )
)]
pub struct ApiDoc;

/// Security scheme configuration
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("Authorization"))),
        );
    }
}

/// Generate OpenAPI JSON specification
pub fn generate_openapi_spec() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap()
}

/// Generate OpenAPI YAML specification
pub fn generate_openapi_yaml() -> String {
    serde_yaml::to_string(&ApiDoc::openapi()).unwrap()
}

/// Swagger UI configuration
pub fn swagger_ui_config() -> &'static str {
    r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>MEVana API v2 - Swagger UI</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@3.52.5/swagger-ui.css" />
    <style>
        html {
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }
        *, *:before, *:after {
            box-sizing: inherit;
        }
        body {
            margin:0;
            background: #fafafa;
        }
        .swagger-ui .topbar {
            background-color: #1f2937;
        }
        .swagger-ui .topbar .download-url-wrapper .download-url-button {
            background: #3b82f6;
        }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@3.52.5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@3.52.5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            const ui = SwaggerUIBundle({
                url: '/api-docs/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout",
                defaultModelsExpandDepth: 1,
                defaultModelExpandDepth: 1,
                docExpansion: "list",
                operationsSorter: "alpha",
                tagsSorter: "alpha",
                validatorUrl: null,
                tryItOutEnabled: true,
                requestInterceptor: function(request) {
                    // Add bearer token if available
                    const token = localStorage.getItem('mevana_auth_token');
                    if (token) {
                        request.headers.Authorization = `Bearer ${token}`;
                    }
                    return request;
                },
                onComplete: function() {
                    // Custom styling
                    const logo = document.querySelector('.topbar-wrapper .link');
                    if (logo) {
                        logo.innerHTML = '<span style="color: white; font-size: 1.5rem; font-weight: bold;">🚀 MEVana API v2</span>';
                        logo.href = 'https://mevana.ai';
                    }
                }
            });
        };
    </script>
</body>
</html>
"#
}

/// ReDoc documentation configuration
pub fn redoc_config() -> &'static str {
    r#"
<!DOCTYPE html>
<html>
<head>
    <title>MEVana API v2 Documentation</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
    <style>
        body {
            margin: 0;
            padding: 0;
        }
    </style>
</head>
<body>
    <redoc spec-url='/api-docs/openapi.json'></redoc>
    <script src="https://cdn.jsdelivr.net/npm/redoc@2.0.0-rc.59/bundles/redoc.standalone.js"></script>
</body>
</html>
"#
}

/// API documentation metadata
pub struct ApiDocumentation {
    pub title: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub base_url: &'static str,
}

impl Default for ApiDocumentation {
    fn default() -> Self {
        Self {
            title: "MEVana API v2",
            version: "2.0.0",
            description: "Enterprise-grade API for MEV arbitrage trading on Solana with advanced analytics, real-time streaming, and institutional-grade security features.",
            base_url: "https://api.mevana.ai",
        }
    }
}

/// Feature flags for API documentation
pub struct ApiFeatures {
    pub enable_auth: bool,
    pub enable_arbitrage: bool,
    pub enable_wallets: bool,
    pub enable_trading: bool,
    pub enable_analytics: bool,
    pub enable_streaming: bool,
    pub enable_admin: bool,
}

impl Default for ApiFeatures {
    fn default() -> Self {
        Self {
            enable_auth: true,
            enable_arbitrage: true,
            enable_wallets: true,
            enable_trading: true,
            enable_analytics: true,
            enable_streaming: true,
            enable_admin: true,
        }
    }
}