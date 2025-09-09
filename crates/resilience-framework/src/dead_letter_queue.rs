use chrono::{DateTime, Utc};
use crossbeam_channel::{bounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLQMessage {
    pub id: String,
    pub payload: serde_json::Value,
    pub error: String,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: DateTime<Utc>,
    pub last_attempted_at: DateTime<Utc>,
    pub source: String,
    pub metadata: std::collections::HashMap<String, String>,
}

impl DLQMessage {
    pub fn new(
        payload: serde_json::Value,
        error: String,
        source: String,
        max_retries: u32,
    ) -> Self {
        let now = Utc::now();
        DLQMessage {
            id: uuid::Uuid::new_v4().to_string(),
            payload,
            error,
            retry_count: 0,
            max_retries,
            created_at: now,
            last_attempted_at: now,
            source,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.last_attempted_at = Utc::now();
    }
    
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
    
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

pub struct DeadLetterQueue {
    queue: Arc<RwLock<VecDeque<DLQMessage>>>,
    channel: (Sender<DLQMessage>, Receiver<DLQMessage>),
    max_size: usize,
    persistence: Option<Arc<dyn DLQPersistence>>,
}

impl DeadLetterQueue {
    pub fn new(max_size: usize) -> Self {
        let (sender, receiver) = bounded(max_size);
        DeadLetterQueue {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            channel: (sender, receiver),
            max_size,
            persistence: None,
        }
    }
    
    pub fn with_persistence(mut self, persistence: Arc<dyn DLQPersistence>) -> Self {
        self.persistence = Some(persistence);
        self
    }
    
    pub async fn push(&self, mut message: DLQMessage) -> Result<(), DLQError> {
        let mut queue = self.queue.write().await;
        
        if queue.len() >= self.max_size {
            return Err(DLQError::QueueFull);
        }
        
        message.increment_retry();
        
        if let Some(persistence) = &self.persistence {
            persistence.save(&message).await?;
        }
        
        queue.push_back(message.clone());
        
        let _ = self.channel.0.try_send(message);
        
        Ok(())
    }
    
    pub async fn pop(&self) -> Option<DLQMessage> {
        let mut queue = self.queue.write().await;
        let message = queue.pop_front();
        
        if let Some(ref msg) = message {
            if let Some(persistence) = &self.persistence {
                let _ = persistence.remove(&msg.id).await;
            }
        }
        
        message
    }
    
    pub async fn peek(&self) -> Option<DLQMessage> {
        let queue = self.queue.read().await;
        queue.front().cloned()
    }
    
    pub async fn size(&self) -> usize {
        self.queue.read().await.len()
    }
    
    pub async fn clear(&self) {
        let mut queue = self.queue.write().await;
        queue.clear();
        
        if let Some(persistence) = &self.persistence {
            let _ = persistence.clear().await;
        }
    }
    
    pub async fn get_messages(&self, limit: usize) -> Vec<DLQMessage> {
        let queue = self.queue.read().await;
        queue.iter().take(limit).cloned().collect()
    }
    
    pub async fn reprocess_message(&self, message_id: &str) -> Result<Option<DLQMessage>, DLQError> {
        let mut queue = self.queue.write().await;
        
        if let Some(pos) = queue.iter().position(|m| m.id == message_id) {
            let mut message = queue.remove(pos).unwrap();
            
            if message.can_retry() {
                message.increment_retry();
                Ok(Some(message))
            } else {
                queue.push_back(message);
                Err(DLQError::MaxRetriesExceeded)
            }
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_statistics(&self) -> DLQStatistics {
        let queue = self.queue.read().await;
        
        let total_messages = queue.len();
        let retryable_messages = queue.iter().filter(|m| m.can_retry()).count();
        let expired_messages = queue.iter().filter(|m| !m.can_retry()).count();
        
        let messages_by_source: std::collections::HashMap<String, usize> = 
            queue.iter()
                .fold(std::collections::HashMap::new(), |mut acc, msg| {
                    *acc.entry(msg.source.clone()).or_insert(0) += 1;
                    acc
                });
        
        DLQStatistics {
            total_messages,
            retryable_messages,
            expired_messages,
            messages_by_source,
            queue_capacity: self.max_size,
            queue_utilization: (total_messages as f64 / self.max_size as f64) * 100.0,
        }
    }
    
    pub fn subscribe(&self) -> Receiver<DLQMessage> {
        self.channel.1.clone()
    }
}

#[derive(Debug, Clone)]
pub struct DLQStatistics {
    pub total_messages: usize,
    pub retryable_messages: usize,
    pub expired_messages: usize,
    pub messages_by_source: std::collections::HashMap<String, usize>,
    pub queue_capacity: usize,
    pub queue_utilization: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum DLQError {
    #[error("Queue is full")]
    QueueFull,
    #[error("Maximum retries exceeded")]
    MaxRetriesExceeded,
    #[error("Persistence error: {0}")]
    PersistenceError(String),
    #[error("Message not found")]
    MessageNotFound,
}

#[async_trait::async_trait]
pub trait DLQPersistence: Send + Sync {
    async fn save(&self, message: &DLQMessage) -> Result<(), DLQError>;
    async fn load(&self) -> Result<Vec<DLQMessage>, DLQError>;
    async fn remove(&self, message_id: &str) -> Result<(), DLQError>;
    async fn clear(&self) -> Result<(), DLQError>;
}

pub struct PostgresDLQPersistence {
    pool: sqlx::PgPool,
}

impl PostgresDLQPersistence {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PostgresDLQPersistence { pool }
    }
}

#[async_trait::async_trait]
impl DLQPersistence for PostgresDLQPersistence {
    async fn save(&self, message: &DLQMessage) -> Result<(), DLQError> {
        sqlx::query(
            "INSERT INTO dead_letter_queue (
                id, payload, error, retry_count, max_retries,
                created_at, last_attempted_at, source, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                retry_count = EXCLUDED.retry_count,
                last_attempted_at = EXCLUDED.last_attempted_at,
                error = EXCLUDED.error"
        )
        .bind(&message.id)
        .bind(&message.payload)
        .bind(&message.error)
        .bind(message.retry_count as i32)
        .bind(message.max_retries as i32)
        .bind(message.created_at)
        .bind(message.last_attempted_at)
        .bind(&message.source)
        .bind(serde_json::to_value(&message.metadata).unwrap())
        .execute(&self.pool)
        .await
        .map_err(|e| DLQError::PersistenceError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn load(&self) -> Result<Vec<DLQMessage>, DLQError> {
        let rows = sqlx::query_as::<_, (
            String, serde_json::Value, String, i32, i32,
            DateTime<Utc>, DateTime<Utc>, String, serde_json::Value
        )>(
            "SELECT id, payload, error, retry_count, max_retries,
                    created_at, last_attempted_at, source, metadata
             FROM dead_letter_queue
             ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DLQError::PersistenceError(e.to_string()))?;
        
        let messages = rows
            .into_iter()
            .map(|row| DLQMessage {
                id: row.0,
                payload: row.1,
                error: row.2,
                retry_count: row.3 as u32,
                max_retries: row.4 as u32,
                created_at: row.5,
                last_attempted_at: row.6,
                source: row.7,
                metadata: serde_json::from_value(row.8).unwrap_or_default(),
            })
            .collect();
        
        Ok(messages)
    }
    
    async fn remove(&self, message_id: &str) -> Result<(), DLQError> {
        sqlx::query("DELETE FROM dead_letter_queue WHERE id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DLQError::PersistenceError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn clear(&self) -> Result<(), DLQError> {
        sqlx::query("DELETE FROM dead_letter_queue")
            .execute(&self.pool)
            .await
            .map_err(|e| DLQError::PersistenceError(e.to_string()))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dlq_push_pop() {
        let dlq = DeadLetterQueue::new(10);
        
        let message = DLQMessage::new(
            serde_json::json!({"test": "data"}),
            "Test error".to_string(),
            "test_source".to_string(),
            3,
        );
        
        dlq.push(message.clone()).await.unwrap();
        assert_eq!(dlq.size().await, 1);
        
        let popped = dlq.pop().await.unwrap();
        assert_eq!(popped.payload, message.payload);
        assert_eq!(dlq.size().await, 0);
    }
    
    #[tokio::test]
    async fn test_dlq_max_size() {
        let dlq = DeadLetterQueue::new(2);
        
        for i in 0..2 {
            let message = DLQMessage::new(
                serde_json::json!({"id": i}),
                "Error".to_string(),
                "source".to_string(),
                3,
            );
            dlq.push(message).await.unwrap();
        }
        
        let message = DLQMessage::new(
            serde_json::json!({"id": 3}),
            "Error".to_string(),
            "source".to_string(),
            3,
        );
        
        let result = dlq.push(message).await;
        assert!(matches!(result, Err(DLQError::QueueFull)));
    }
}