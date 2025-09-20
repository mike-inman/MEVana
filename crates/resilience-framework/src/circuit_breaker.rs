use governor::{Quota, RateLimiter};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open(Instant),
    HalfOpen,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_requests: u32,
    pub rate_limit: Option<u32>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_requests: 3,
            rate_limit: Some(100),
        }
    }
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    half_open_requests: Arc<RwLock<u32>>,
    rate_limiter: Option<
        Arc<
            RateLimiter<
                governor::state::direct::NotKeyed,
                governor::state::InMemoryState,
                governor::clock::DefaultClock,
            >,
        >,
    >,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        let rate_limiter = config.rate_limit.map(|limit| {
            Arc::new(RateLimiter::direct(Quota::per_second(
                std::num::NonZeroU32::new(limit).unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
            )))
        });

        CircuitBreaker {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            half_open_requests: Arc::new(RwLock::new(0)),
            rate_limiter,
        }
    }

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        if let Some(limiter) = &self.rate_limiter {
            limiter.until_ready().await;
        }

        let state = self.state.read().await.clone();

        match state {
            CircuitState::Open(opened_at) => {
                if Instant::now().duration_since(opened_at) >= self.config.timeout {
                    *self.state.write().await = CircuitState::HalfOpen;
                    *self.half_open_requests.write().await = 0;
                    log::info!("Circuit breaker transitioning to half-open");
                } else {
                    return Err(CircuitBreakerError::CircuitOpen);
                }
            }
            CircuitState::HalfOpen => {
                let mut requests = self.half_open_requests.write().await;
                if *requests >= self.config.half_open_requests {
                    return Err(CircuitBreakerError::CircuitOpen);
                }
                *requests += 1;
            }
            CircuitState::Closed => {}
        }

        match operation.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(CircuitBreakerError::OperationFailed(error.to_string()))
            }
        }
    }

    async fn on_success(&self) {
        let state = self.state.read().await.clone();

        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    *success_count = 0;
                    log::info!("Circuit breaker closed after successful recovery");
                }
            }
            CircuitState::Closed => {
                *self.failure_count.write().await = 0;
            }
            _ => {}
        }
    }

    async fn on_failure(&self) {
        let state = self.state.read().await.clone();

        match state {
            CircuitState::HalfOpen => {
                *self.state.write().await = CircuitState::Open(Instant::now());
                *self.success_count.write().await = 0;
                log::warn!("Circuit breaker opened due to failure in half-open state");
            }
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;

                if *failure_count >= self.config.failure_threshold {
                    *self.state.write().await = CircuitState::Open(Instant::now());
                    log::warn!("Circuit breaker opened after {} failures", *failure_count);
                }
            }
            _ => {}
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        *self.half_open_requests.write().await = 0;
    }

    pub async fn force_open(&self) {
        *self.state.write().await = CircuitState::Open(Instant::now());
    }

    pub async fn get_metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            state: self.state.read().await.clone(),
            failure_count: *self.failure_count.read().await,
            success_count: *self.success_count.read().await,
        }
    }
}

#[derive(Debug)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open")]
    CircuitOpen,
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

pub struct MultiCircuitBreaker {
    breakers: std::collections::HashMap<String, Arc<CircuitBreaker>>,
}

impl MultiCircuitBreaker {
    pub fn new() -> Self {
        MultiCircuitBreaker {
            breakers: std::collections::HashMap::new(),
        }
    }

    pub fn add_breaker(&mut self, name: String, config: CircuitBreakerConfig) {
        self.breakers
            .insert(name, Arc::new(CircuitBreaker::new(config)));
    }

    pub fn get_breaker(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        self.breakers.get(name).cloned()
    }

    pub async fn call<F, T, E>(
        &self,
        breaker_name: &str,
        operation: F,
    ) -> Result<T, CircuitBreakerError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        match self.get_breaker(breaker_name) {
            Some(breaker) => breaker.call(operation).await,
            None => operation
                .await
                .map_err(|e| CircuitBreakerError::OperationFailed(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };

        let breaker = CircuitBreaker::new(config);

        let _ = breaker.call(async { Err::<(), _>("error") }).await;
        let _ = breaker.call(async { Err::<(), _>("error") }).await;

        assert_eq!(
            breaker.get_state().await,
            CircuitState::Open(Instant::now())
        );
    }

    #[tokio::test]
    async fn test_circuit_closes_after_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };

        let breaker = CircuitBreaker::new(config);

        let _ = breaker.call(async { Err::<(), _>("error") }).await;
        assert!(matches!(breaker.get_state().await, CircuitState::Open(_)));

        tokio::time::sleep(Duration::from_millis(150)).await;

        let _ = breaker.call(async { Ok::<_, &str>(()) }).await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }
}
