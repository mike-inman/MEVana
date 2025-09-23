use crate::config::{parse_pubkey, providers_config, ProviderConfig, ProviderReserveConfig};
use crate::plugin::{
    Environment, FlashLoanProvider, PlanInstructions, PlanRequest, PlanResponse, ProviderId,
    QuoteRequest, QuoteResponse,
};
use crate::registry::register_provider;
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use flash_loan_receiver::build_execute_flash_loan_ix;
use log::{info, warn};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use std::borrow::Cow;
use std::sync::Arc;

pub const PROVIDER_ID: ProviderId = ProviderId::new("kamino");
const DEFAULT_MAX_LOAN_LAMPORTS: u64 = 100_000_000_000; // 100 SOL
const DEFAULT_FEE_BPS: u16 = 1; // 0.01%
const DEFAULT_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);

#[derive(Clone, Debug)]
pub struct KaminoProvider;

impl KaminoProvider {
    pub fn register() {
        if register_provider(Arc::new(Self)) {
            info!("Registered Kamino flash loan provider");
        }
    }

    fn config_for_mint<'a>(
        cfg: &'a ProviderConfig,
        mint: &Pubkey,
    ) -> Option<&'a ProviderReserveConfig> {
        cfg.reserves.get(&mint.to_string())
    }

    fn default_program_id() -> Pubkey {
        providers_config()
            .kamino
            .program_id
            .as_ref()
            .and_then(|pid| pid.parse().ok())
            .unwrap_or(DEFAULT_PROGRAM_ID)
    }
}

#[async_trait]
impl FlashLoanProvider for KaminoProvider {
    fn id(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn program_id(&self) -> Pubkey {
        Self::default_program_id()
    }

    fn display_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Kamino")
    }

    fn supports_token(&self, mint: &Pubkey) -> bool {
        let cfg = providers_config();
        Self::config_for_mint(&cfg.kamino, mint).is_some()
    }

    async fn quote(
        &self,
        _environment: &Environment,
        request: &QuoteRequest,
    ) -> Result<Option<QuoteResponse>> {
        let cfg = providers_config();
        let Some(reserve_cfg) = Self::config_for_mint(&cfg.kamino, &request.token_mint) else {
            warn!(
                "Kamino reserve configuration missing for mint {}",
                request.token_mint
            );
            return Ok(None);
        };

        let max_available = reserve_cfg
            .available_amount
            .unwrap_or(DEFAULT_MAX_LOAN_LAMPORTS);
        if max_available == 0 {
            return Ok(None);
        }

        let amount = request.amount.min(max_available);
        if amount == 0 {
            return Ok(None);
        }

        let fee_bps = reserve_cfg.fee_bps.unwrap_or(DEFAULT_FEE_BPS);
        let fee_lamports = reserve_cfg
            .fee_lamports
            .unwrap_or(((amount as u128 * fee_bps as u128) / 10_000) as u64);

        Ok(Some(QuoteResponse {
            provider: PROVIDER_ID,
            token_mint: request.token_mint,
            available_amount: amount,
            fee_bps,
            fee_lamports,
            estimated_latency_ms: 120,
            confidence: 0.7,
        }))
    }

    async fn build_plan(
        &self,
        _environment: &Environment,
        request: &PlanRequest,
    ) -> Result<PlanResponse> {
        let cfg = providers_config();
        let Some(reserve_cfg) = Self::config_for_mint(&cfg.kamino, &request.token_mint) else {
            bail!(
                "Kamino reserve configuration missing for mint {}",
                request.token_mint
            );
        };

        let max_available = reserve_cfg
            .available_amount
            .unwrap_or(DEFAULT_MAX_LOAN_LAMPORTS);
        let amount = request.amount.min(max_available);
        if amount == 0 {
            bail!("flash loan amount must be greater than zero");
        }

        let fee_lamports = reserve_cfg.fee_lamports.unwrap_or(
            ((amount as u128 * reserve_cfg.fee_bps.unwrap_or(DEFAULT_FEE_BPS) as u128) / 10_000)
                as u64,
        );

        let receiver_program = request
            .receiver_program_id
            .or_else(|| {
                cfg.kamino
                    .program_id
                    .as_ref()
                    .and_then(|pid| pid.parse().ok())
            })
            .unwrap_or_else(Self::default_program_id);

        let reserve = request
            .reserve_hint
            .or_else(|| parse_pubkey(&reserve_cfg.reserve))
            .ok_or_else(|| anyhow!("Kamino reserve account missing in configuration"))?;

        let liquidity_vault = parse_pubkey(&reserve_cfg.liquidity_vault)
            .ok_or_else(|| anyhow!("Kamino liquidity vault missing in configuration"))?;

        let fee_receiver = parse_pubkey(&reserve_cfg.fee_receiver)
            .ok_or_else(|| anyhow!("Kamino fee receiver missing in configuration"))?;

        let mut extra_accounts = vec![
            AccountMeta::new_readonly(reserve, false),
            AccountMeta::new_readonly(liquidity_vault, false),
            AccountMeta::new_readonly(fee_receiver, false),
        ];

        if let Some(host) = parse_pubkey(&reserve_cfg.host_fee_receiver) {
            extra_accounts.push(AccountMeta::new_readonly(host, false));
        }

        let execute_ix = build_execute_flash_loan_ix(
            &receiver_program,
            request.borrower,
            PROVIDER_ID.as_str(),
            request.token_mint,
            amount,
            extra_accounts,
            request.receiver_payload.clone().unwrap_or_default(),
        )?;

        let mut instructions = PlanInstructions::new();
        instructions.borrow.push(execute_ix);

        Ok(PlanResponse {
            provider: PROVIDER_ID,
            amount,
            fee_lamports,
            instructions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        set_providers_config_for_tests, FlashLoanProvidersConfig, ProviderConfig,
        ProviderReserveConfig,
    };
    use crate::plugin::{Environment, PlanRequest, QuoteRequest};
    use solana_sdk::pubkey::Pubkey;
    use std::collections::HashMap;
    use std::str::FromStr;

    fn setup_config() -> FlashLoanProvidersConfig {
        let reserve_key = Pubkey::new_unique();
        let liquidity_vault = Pubkey::new_unique();
        let fee_receiver = Pubkey::new_unique();
        let host_fee = Pubkey::new_unique();

        let reserve = ProviderReserveConfig {
            available_amount: Some(5_000_000_000),
            fee_bps: Some(1),
            fee_lamports: Some(50_000),
            reserve: Some(reserve_key.to_string()),
            liquidity_vault: Some(liquidity_vault.to_string()),
            fee_receiver: Some(fee_receiver.to_string()),
            host_fee_receiver: Some(host_fee.to_string()),
        };

        let mut reserves = HashMap::new();
        reserves.insert(
            Pubkey::from_str("So11111111111111111111111111111111111111112")
                .unwrap()
                .to_string(),
            reserve,
        );

        let kamino = ProviderConfig {
            reserves,
            program_id: Some(DEFAULT_PROGRAM_ID.to_string()),
        };

        FlashLoanProvidersConfig {
            kamino,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn kamino_quote_and_plan_from_config() {
        let cfg = setup_config();
        set_providers_config_for_tests(cfg);

        let provider = KaminoProvider;
        let wsol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let borrower = Pubkey::new_unique();

        let quote = provider
            .quote(
                &Environment::default(),
                &QuoteRequest {
                    token_mint: wsol,
                    amount: 1_000_000_000,
                    reserve_hint: None,
                },
            )
            .await
            .expect("quote ok")
            .expect("quote available");

        assert_eq!(quote.provider, PROVIDER_ID);
        assert_eq!(quote.available_amount, 1_000_000_000);
        assert_eq!(quote.fee_lamports, 50_000);

        let plan = provider
            .build_plan(
                &Environment::default(),
                &PlanRequest {
                    token_mint: wsol,
                    amount: 500_000_000,
                    borrower,
                    receiver_program_id: None,
                    receiver_payload: Some(vec![1, 2, 3]),
                    reserve_hint: None,
                },
            )
            .await
            .expect("plan built");

        assert_eq!(plan.provider, PROVIDER_ID);
        assert_eq!(plan.amount, 500_000_000);
        assert_eq!(plan.instructions.borrow.len(), 1);
        assert!(plan.instructions.repay.is_empty());

        // reset global config to default to avoid affecting other tests
        set_providers_config_for_tests(FlashLoanProvidersConfig::default());
    }
}
