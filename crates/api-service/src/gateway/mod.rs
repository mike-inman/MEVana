//! Gateway module scaffolding for consolidating UI-facing traffic.

pub mod config;
pub mod fanout;
pub mod proxy;
pub mod routes;
pub mod sse;

pub use config::GatewayConfig;
pub use fanout::{GatewayConsumerHandles, GatewayFanout};
pub use proxy::DownstreamProxy;
pub use routes::{build_gateway_router, GatewayContext};
