use std::sync::Arc;

use super::config::GatewayConfig;

/// Placeholder proxy for downstream service requests.
///
/// The real implementation will forward REST/WebSocket calls to services such as
/// `arb-engine`, `hotpath_arb_engine`, and `monitoring-service`. For now we keep
/// the structure lightweight so future patches can fill in the behaviour without
/// reshaping public types again.
#[derive(Debug, Clone)]
pub struct DownstreamProxy {
    config: Arc<GatewayConfig>,
}

impl DownstreamProxy {
    pub fn new(config: Arc<GatewayConfig>) -> Self {
        Self { config }
    }

    /// Returns a short descriptor that can be used in health responses while
    /// the proxy is still a stub.
    pub fn readiness_status(&self) -> String {
        format!(
            "proxy-initialized:kafka={} prices_topic={}",
            self.config.kafka_broker, self.config.prices_topic
        )
    }
}
