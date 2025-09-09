//! API routes configuration

use axum::Router;

/// Create API routes
pub fn create_api_routes() -> Router {
    Router::new()
        // Routes would be defined here
        // .route("/auth/login", post(handlers::auth::login))
        // .route("/users", get(handlers::users::list_users))
        // etc.
}

/// Create documentation routes
pub fn create_docs_routes() -> Router {
    Router::new()
        // OpenAPI documentation routes would be here
}