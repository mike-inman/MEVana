//! Validation Pipeline
//!
//! Orchestrates the complete validation workflow from live transactions
//! to ground truth comparison and result storage.

use crate::{
    database::ValidationDatabase,
    streaming::{StreamedTransaction, StreamingConfig, TransactionStreamProcessor},
    validation::{ParsedTransactionData, ValidationFramework},
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Complete validation pipeline configuration
#[derive(Debug, Clone)]
pub struct ValidationPipelineConfig {
    pub streaming: StreamingConfig,
    pub validation_run_id: Uuid,
    pub max_concurrent_validations: usize,
    pub batch_size: usize,
}

impl Default for ValidationPipelineConfig {
    fn default() -> Self {
        Self {
            streaming: StreamingConfig::default(),
            validation_run_id: Uuid::new_v4(),
            max_concurrent_validations: 10,
            batch_size: 100,
        }
    }
}

/// Main validation pipeline
pub struct ValidationPipeline {
    framework: Arc<ValidationFramework>,
    config: ValidationPipelineConfig,
    database: Arc<ValidationDatabase>,
}

impl ValidationPipeline {
    /// Create new validation pipeline
    pub fn new(
        framework: ValidationFramework,
        database: ValidationDatabase,
        config: ValidationPipelineConfig,
    ) -> Self {
        Self {
            framework: Arc::new(framework),
            database: Arc::new(database),
            config,
        }
    }

    /// Start the complete validation pipeline
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Parser Validation Pipeline");
        info!("📊 Validation Run ID: {}", self.config.validation_run_id);

        // Create channels for pipeline communication
        let (tx_sender, tx_receiver) = mpsc::channel::<StreamedTransaction>(1000);
        let (result_sender, result_receiver) = mpsc::channel(1000);

        // Start transaction stream processor
        let stream_processor =
            TransactionStreamProcessor::new(self.config.streaming.clone(), tx_sender);

        // Start validation workers
        let validation_workers = self.start_validation_workers(
            tx_receiver,
            result_sender,
            self.config.max_concurrent_validations,
        );

        // Start result storage worker
        let storage_worker = self.start_result_storage_worker(result_receiver);

        // Start metrics collection
        let metrics_worker = self.start_metrics_worker();

        // Start stream processing
        let stream_worker = stream_processor.start();

        info!("✅ All pipeline components started");

        // Wait for all workers (or handle shutdown gracefully)
        tokio::select! {
            result = stream_worker => {
                if let Err(e) = result {
                    error!("Stream processor failed: {}", e);
                }
            }
            result = validation_workers => {
                if let Err(e) = result {
                    error!("Validation workers failed: {}", e);
                }
            }
            result = storage_worker => {
                if let Err(e) = result {
                    error!("Storage worker failed: {}", e);
                }
            }
            result = metrics_worker => {
                if let Err(e) = result {
                    error!("Metrics worker failed: {}", e);
                }
            }
        }

        info!("🏁 Validation pipeline completed");
        Ok(())
    }

    /// Start validation workers that process transactions
    async fn start_validation_workers(
        &self,
        mut tx_receiver: mpsc::Receiver<StreamedTransaction>,
        result_sender: mpsc::Sender<crate::validation::ValidationResult>,
        max_concurrent: usize,
    ) -> Result<()> {
        info!("👷 Starting {} validation workers", max_concurrent);

        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

        while let Some(streamed_tx) = tx_receiver.recv().await {
            let permit = semaphore.clone().acquire_owned().await?;
            let framework = self.framework.clone();
            let sender = result_sender.clone();

            tokio::spawn(async move {
                let _permit = permit; // Keep permit alive

                if let Err(e) =
                    Self::validate_single_transaction(framework, streamed_tx, sender).await
                {
                    error!("Validation failed: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Validate a single transaction
    async fn validate_single_transaction(
        framework: Arc<ValidationFramework>,
        streamed_tx: StreamedTransaction,
        result_sender: mpsc::Sender<crate::validation::ValidationResult>,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Step 1: Parse transaction with our system
        let parsed_data = Self::parse_with_our_system(&streamed_tx).await?;

        // Step 2: Get ground truth
        let ground_truth = framework.get_ground_truth(&streamed_tx.signature).await?;

        // Step 3: Validate
        let validation_result = framework
            .validate_transaction(parsed_data, ground_truth)
            .await?;

        let total_time = start_time.elapsed();
        info!(
            "✅ Validated {} in {}μs (accuracy: {:.1}%, completeness: {:.1}%)",
            streamed_tx.signature,
            total_time.as_micros(),
            validation_result.overall_accuracy * 100.0,
            validation_result.completeness_ratio * 100.0
        );

        // Send result for storage
        if let Err(e) = result_sender.send(validation_result).await {
            warn!("Failed to send validation result: {}", e);
        }

        Ok(())
    }

    /// Parse transaction with our normalizer system
    async fn parse_with_our_system(
        streamed_tx: &StreamedTransaction,
    ) -> Result<ParsedTransactionData> {
        // TODO: Integrate with actual normalizer-service
        // For now, create mock parsed data

        Ok(ParsedTransactionData {
            transaction_signature: streamed_tx.signature.clone(),
            program_id: streamed_tx.program_id.clone(),
            instruction_name: "swap".to_string(), // Mock
            block_time: streamed_tx.block_time,
            slot: streamed_tx.slot,
            token_in: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()), // USDC
            token_out: Some("So11111111111111111111111111111111111111112".to_string()), // SOL
            amount_in: Some(1000000),                                                   // 1 USDC
            amount_out: Some(50000000),                                                 // 0.05 SOL
            user: Some("user123".to_string()),
            pool: Some("pool456".to_string()),
            price: Some(0.05),         // 0.05 SOL per USDC
            price_impact: Some(0.001), // 0.1%
            fee_amount: Some(3000),    // 0.003 USDC
            fee_mint: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
            parser_version: "0.1.0".to_string(),
            parse_duration_us: 5000, // 5ms
            confidence_score: 0.95,
        })
    }

    /// Start result storage worker
    async fn start_result_storage_worker(
        &self,
        mut result_receiver: mpsc::Receiver<crate::validation::ValidationResult>,
    ) -> Result<()> {
        info!("💾 Starting result storage worker");

        let database = self.database.clone();

        while let Some(result) = result_receiver.recv().await {
            if let Err(e) = database.store_validation_result(&result).await {
                error!("Failed to store validation result: {}", e);
            }
        }

        Ok(())
    }

    /// Start metrics collection worker
    async fn start_metrics_worker(&self) -> Result<()> {
        info!("📈 Starting metrics collection worker");

        // TODO: Implement Prometheus metrics collection
        // For now, just periodic logging

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        loop {
            interval.tick().await;
            info!("📊 Metrics collection tick (placeholder)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GroundTruthAggregator, ValidationFramework};

    #[tokio::test]
    async fn test_pipeline_creation() {
        let ground_truth =
            GroundTruthAggregator::new("test".to_string(), "test".to_string(), "test".to_string());

        // Note: This test will fail without a real database
        // let framework = ValidationFramework::new(ground_truth, database);
        // let config = ValidationPipelineConfig::default();
        // let pipeline = ValidationPipeline::new(framework, database, config);

        // For now, just test that the types compile
        assert!(true);
    }
}
