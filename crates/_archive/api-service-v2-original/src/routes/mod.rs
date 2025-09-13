//! API routes configuration

use axum::Router;

/// Create API routes
pub fn create_api_routes() -> Router {
    Router::new()
        // Arbitrage routes
        .route("/arbitrage/opportunities", 
               axum::routing::post(crate::handlers::create_arbitrage_opportunity)
               .get(crate::handlers::get_arbitrage_opportunities))
}

/// Create documentation routes
pub fn create_docs_routes() -> Router {
    Router::new()
        // OpenAPI documentation routes would be here
}