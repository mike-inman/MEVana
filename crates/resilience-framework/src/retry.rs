use async_trait::async_trait;
use backoff::ExponentialBackoff;
use std::future::Future;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RetryError {
    #[error("Maximum retries exceeded: {0}")]
    MaxRetriesExceeded(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Non-retryable error: {0}")]
    NonRetryable(String),
}

#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_interval: Duration,
    pub max_interval: Duration,
    pub exponential_base: f64,
    pub max_elapsed_time: Option<Duration>,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_retries: 3,
            initial_interval: Duration::from_millis(100),
            max_interval: Duration::from_secs(30),
            exponential_base: 2.0,
            max_elapsed_time: Some(Duration::from_secs(300)),
            jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn aggressive() -> Self {
        RetryPolicy {
            max_retries: 10,
            initial_interval: Duration::from_millis(50),
            max_interval: Duration::from_secs(5),
            exponential_base: 1.5,
            max_elapsed_time: Some(Duration::from_secs(60)),
            jitter: true,
        }
    }
    
    pub fn conservative() -> Self {
        RetryPolicy {
            max_retries: 5,
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(60),
            exponential_base: 3.0,
            max_elapsed_time: Some(Duration::from_secs(600)),
            jitter: false,
        }
    }
    
    pub fn transaction_optimized() -> Self {
        RetryPolicy {
            max_retries: 5,
            initial_interval: Duration::from_millis(200),
            max_interval: Duration::from_secs(10),
            exponential_base: 2.0,
            max_elapsed_time: Some(Duration::from_secs(30)),
            jitter: true,
        }
    }
}

#[async_trait]
pub trait RetryableOperation {
    type Output;
    type Error;
    
    async fn execute(&mut self) -> Result<Self::Output, Self::Error>;
    
    fn is_retryable(&self, _error: &Self::Error) -> bool {
        true
    }
    
    fn on_retry(&mut self, attempt: u32, _error: &Self::Error) {
        log::warn!("Retry attempt {}", attempt);
    }
}

pub struct RetryExecutor {
    policy: RetryPolicy,
}

impl RetryExecutor {
    pub fn new(policy: RetryPolicy) -> Self {
        RetryExecutor { policy }
    }
    
    pub async fn execute<F, T, E>(&self, mut operation: F) -> Result<T, RetryError>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
        E: std::fmt::Display,
    {
        let backoff = self.create_backoff();
        let mut attempts = 0;
        
        loop {
            attempts += 1;
            
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    let error_msg = error.to_string();
                    
                    if attempts >= self.policy.max_retries {
                        return Err(RetryError::MaxRetriesExceeded(error_msg));
                    }
                    
                    log::warn!(
                        "Operation failed (attempt {}/{}): {}",
                        attempts, self.policy.max_retries, error_msg
                    );
                    
                    let delay = backoff.initial_interval.min(backoff.max_interval);
                    log::debug!("Retrying in {:?}", delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    pub async fn execute_with_operation<O>(&self, mut operation: O) -> Result<O::Output, RetryError>
    where
        O: RetryableOperation,
        O::Error: std::fmt::Display,
    {
        let backoff = self.create_backoff();
        let mut attempts = 0;
        
        loop {
            attempts += 1;
            
            match operation.execute().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    let error_msg = error.to_string();
                    
                    if !operation.is_retryable(&error) {
                        return Err(RetryError::NonRetryable(error_msg));
                    }
                    
                    if attempts >= self.policy.max_retries {
                        return Err(RetryError::MaxRetriesExceeded(error_msg));
                    }
                    
                    operation.on_retry(attempts, &error);
                    
                    let delay = backoff.initial_interval.min(backoff.max_interval);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    fn create_backoff(&self) -> ExponentialBackoff {
        let mut backoff = ExponentialBackoff::default();
        backoff.initial_interval = self.policy.initial_interval;
        backoff.max_interval = self.policy.max_interval;
        backoff.multiplier = self.policy.exponential_base;
        backoff.max_elapsed_time = self.policy.max_elapsed_time;
        
        if !self.policy.jitter {
            backoff.randomization_factor = 0.0;
        }
        
        backoff
    }
}

pub struct IdempotentRetry<T> {
    idempotency_key: String,
    operation: T,
    completed_operations: std::sync::Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
}

impl<T> IdempotentRetry<T> {
    pub fn new(idempotency_key: String, operation: T) -> Self {
        IdempotentRetry {
            idempotency_key,
            operation,
            completed_operations: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashSet::new()
            )),
        }
    }
    
    pub async fn execute<F, O, E>(&mut self, mut f: F) -> Result<O, E>
    where
        F: FnMut(&mut T) -> std::pin::Pin<Box<dyn Future<Output = Result<O, E>> + Send>>,
    {
        let completed = self.completed_operations.read().await;
        if completed.contains(&self.idempotency_key) {
            log::info!("Operation {} already completed, skipping", self.idempotency_key);
            return Ok(unsafe { std::mem::zeroed() });
        }
        drop(completed);
        
        let result = f(&mut self.operation).await?;
        
        let mut completed = self.completed_operations.write().await;
        completed.insert(self.idempotency_key.clone());
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let mut attempt = 0;
        let policy = RetryPolicy {
            max_retries: 3,
            initial_interval: Duration::from_millis(10),
            ..Default::default()
        };
        
        let executor = RetryExecutor::new(policy);
        let result = executor.execute(|| {
            attempt += 1;
            Box::pin(async move {
                if attempt == 1 {
                    Err("First attempt failed")
                } else {
                    Ok("Success")
                }
            })
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt, 2);
    }
    
    #[tokio::test]
    async fn test_retry_max_attempts_exceeded() {
        let policy = RetryPolicy {
            max_retries: 2,
            initial_interval: Duration::from_millis(10),
            ..Default::default()
        };
        
        let executor = RetryExecutor::new(policy);
        let result = executor.execute(|| {
            Box::pin(async move {
                Err::<(), _>("Always fails")
            })
        }).await;
        
        assert!(matches!(result, Err(RetryError::MaxRetriesExceeded(_))));
    }
}