use crate::config::providers_config;
use crate::plugin::{
    Environment, FlashLoanProvider, PlanInstructions, PlanRequest, PlanResponse, ProviderId,
    QuoteRequest, QuoteResponse,
};
use crate::registry::register_provider;
use anyhow::{bail, Result};
use async_trait::async_trait;
use log::info;
use solana_sdk::pubkey::Pubkey;
use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

pub mod instructions;

use instructions::{
    calculate_flash_loan_fee, create_flash_loan_instruction, create_flash_loan_repay_instruction,
    validate_flash_loan_preconditions, MARGINFI_PROGRAM_ID,
};

pub const PROVIDER_ID: ProviderId = ProviderId::new("marginfi");
const MAX_LOAN_LAMPORTS: u64 = 50_000_000_000; // 50 SOL cap until dynamic data available
const SUPPORTED_SOL_MINT: Pubkey =
    solana_sdk::pubkey!("So11111111111111111111111111111111111111112");

#[derive(Default)]
pub struct MarginfiProvider;

impl MarginfiProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn register() {
        let provider = Arc::new(Self::new());
        if register_provider(provider.clone()) {
            info!("Registered Marginfi flash loan provider");
        }
    }
}

impl fmt::Debug for MarginfiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarginfiProvider").finish()
    }
}

#[async_trait]
impl FlashLoanProvider for MarginfiProvider {
    fn id(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn program_id(&self) -> Pubkey {
        MARGINFI_PROGRAM_ID
    }

    fn display_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Marginfi")
    }

    fn supports_token(&self, mint: &Pubkey) -> bool {
        *mint == SUPPORTED_SOL_MINT
    }

    fn max_loan_amount(&self) -> Option<u64> {
        Some(MAX_LOAN_LAMPORTS)
    }

    async fn quote(
        &self,
        _environment: &Environment,
        request: &QuoteRequest,
    ) -> Result<Option<QuoteResponse>> {
        if !self.supports_token(&request.token_mint) {
            return Ok(None);
        }

        let cfg = providers_config();
        let reserve_cfg = cfg.marginfi.reserves.get(&request.token_mint.to_string());
        let capped_amount = request.amount.min(
            reserve_cfg
                .and_then(|c| c.available_amount)
                .unwrap_or(MAX_LOAN_LAMPORTS),
        );
        if capped_amount == 0 {
            return Ok(None);
        }
        let fee_bps = reserve_cfg.and_then(|c| c.fee_bps).unwrap_or(5);
        let fee_lamports = reserve_cfg
            .and_then(|c| c.fee_lamports)
            .unwrap_or_else(|| calculate_flash_loan_fee(capped_amount));

        Ok(Some(QuoteResponse {
            provider: PROVIDER_ID,
            token_mint: request.token_mint,
            available_amount: capped_amount,
            fee_bps,
            fee_lamports,
            estimated_latency_ms: 120,
            confidence: 0.85,
        }))
    }

    async fn build_plan(
        &self,
        _environment: &Environment,
        request: &PlanRequest,
    ) -> Result<PlanResponse> {
        if !self.supports_token(&request.token_mint) {
            bail!("Unsupported token for Marginfi flash loan");
        }

        let cfg = providers_config();
        let reserve_cfg = cfg.marginfi.reserves.get(&request.token_mint.to_string());
        let capped_amount = request.amount.min(
            reserve_cfg
                .and_then(|c| c.available_amount)
                .unwrap_or(MAX_LOAN_LAMPORTS),
        );
        validate_flash_loan_preconditions(capped_amount, MAX_LOAN_LAMPORTS)?;

        let fee = reserve_cfg
            .and_then(|c| c.fee_lamports)
            .unwrap_or_else(|| calculate_flash_loan_fee(capped_amount));

        let program_id = cfg
            .marginfi
            .program_id
            .as_ref()
            .and_then(|pid| pid.parse().ok())
            .unwrap_or(MARGINFI_PROGRAM_ID);

        let borrow_ix = create_flash_loan_instruction(&request.borrower, capped_amount)?;
        let repay_ix = create_flash_loan_repay_instruction(&request.borrower, capped_amount + fee)?;

        let mut instructions = PlanInstructions::new();
        instructions.borrow.push(borrow_ix);
        instructions.repay.push(repay_ix);

        let mut response = PlanResponse {
            provider: PROVIDER_ID,
            amount: capped_amount,
            fee_lamports: fee,
            instructions,
        };

        if program_id != MARGINFI_PROGRAM_ID {
            // Update program id in instructions if override provided
            for ix in &mut response.instructions.borrow {
                ix.program_id = program_id;
            }
            for ix in &mut response.instructions.repay {
                ix.program_id = program_id;
            }
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{Environment, PlanRequest, QuoteRequest};
    use solana_sdk::pubkey::Pubkey;

    #[tokio::test]
    async fn marginfi_quote_and_plan_for_wsol() {
        let provider = MarginfiProvider::new();
        let borrower = Pubkey::new_unique();

        let quote = provider
            .quote(
                &Environment::default(),
                &QuoteRequest {
                    token_mint: SUPPORTED_SOL_MINT,
                    amount: 20_000_000_000,
                    reserve_hint: None,
                },
            )
            .await
            .expect("quote succeeds")
            .expect("quote available");

        assert_eq!(quote.provider, PROVIDER_ID);
        assert_eq!(quote.available_amount, 20_000_000_000);
        assert_eq!(quote.fee_lamports, calculate_flash_loan_fee(20_000_000_000));

        let plan = provider
            .build_plan(
                &Environment::default(),
                &PlanRequest {
                    token_mint: SUPPORTED_SOL_MINT,
                    amount: 10_000_000_000,
                    borrower,
                    receiver_program_id: None,
                    receiver_payload: None,
                    reserve_hint: None,
                },
            )
            .await
            .expect("plan builds");

        assert_eq!(plan.provider, PROVIDER_ID);
        assert_eq!(plan.instructions.borrow.len(), 1);
        assert_eq!(plan.instructions.repay.len(), 1);
        assert!(plan.fee_lamports > 0);
    }

    #[tokio::test]
    async fn marginfi_rejects_small_amounts() {
        let provider = MarginfiProvider::new();
        let borrower = Pubkey::new_unique();
        let result = provider
            .build_plan(
                &Environment::default(),
                &PlanRequest {
                    token_mint: SUPPORTED_SOL_MINT,
                    amount: 1000,
                    borrower,
                    receiver_program_id: None,
                    receiver_payload: None,
                    reserve_hint: None,
                },
            )
            .await;
        assert!(result.is_err());
    }
}
