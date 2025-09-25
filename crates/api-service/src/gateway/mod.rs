//! Gateway module scaffolding for consolidating UI-facing traffic.

pub mod config;
pub mod proxy;
pub mod routes;
pub mod websocket;

pub use config::GatewayConfig;
pub use proxy::DownstreamProxy;
pub use routes::{build_gateway_router, GatewayContext};
pub use websocket::{GatewayConsumerHandles, GatewayWebsocketManager};
