use crate::plugin::{
    Environment, FlashLoanProvider, PlanInstructions, PlanRequest, PlanResponse, ProviderId,
    QuoteRequest, QuoteResponse,
};
use crate::registry::register_provider;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use flash_loan_receiver::build_execute_flash_loan_ix;
use log::info;
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};
use std::borrow::Cow;
use std::env;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

const PORT_FINANCE_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("Port7uDYB3wk6GJAw4KT1WpTeMtSu9bTcChBHkX2LfR");
const WSOL_MINT: Pubkey = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");
const DEVNET_LENDING_MARKET: Pubkey =
    solana_sdk::pubkey!("3ap6AVQUNTTKkxxizjJM7QfMSzBdYdHrheVF5A8YbopT");
const DEVNET_RESERVE: Pubkey = solana_sdk::pubkey!("HWwkuqTst4CNBdkLDgsUY2K2pijn6MDxVGpxdATn91BA");
const DEVNET_RESERVE_LIQUIDITY_SUPPLY: Pubkey =
    solana_sdk::pubkey!("9Rue5zQjHnfzffvxB1XtgFcB84yKfFzN5LnGMD4SusVD");
const DEVNET_FEE_RECEIVER: Pubkey =
    solana_sdk::pubkey!("4uuDuHvpTS8hShhkYjmYqNimz9HAZH5eZVDvYuBTGXFM");

const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const MAX_LOAN_LAMPORTS: u64 = 30_000_000_000; // 30 SOL cap for devnet setup
const FLASH_LOAN_FEE_BPS: u16 = 12; // 0.12%

pub const PROVIDER_ID: ProviderId = ProviderId::new("portfinance");

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct PortFinanceReserveConfig {
    program_id: Pubkey,
    lending_market: Pubkey,
    market_authority: Pubkey,
    reserve: Pubkey,
    reserve_liquidity_supply: Pubkey,
    fee_receiver: Pubkey,
    token_program_id: Pubkey,
}

impl PortFinanceReserveConfig {
    fn validate(&self) -> Result<()> {
        let (derived_authority, _) =
            Pubkey::find_program_address(&[self.lending_market.as_ref()], &self.program_id);
        if derived_authority != self.market_authority {
            return Err(anyhow!(
                "port finance config mismatch: derived authority {} != provided {}",
                derived_authority,
                self.market_authority
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct PortFinanceProvider;

impl PortFinanceProvider {
    fn new() -> Self {
        Self
    }

    pub fn register() {
        let cfg = devnet_config();

        if let Err(err) = cfg.validate() {
            info!("Skipping Port Finance registration: {err:?}");
            return;
        }

        let provider = Arc::new(Self::new());
        if register_provider(provider) {
            info!("Registered Port Finance flash loan provider (WSOL devnet)");
        }
    }

    fn compute_fee_lamports(amount: u64) -> u64 {
        (amount as u128 * FLASH_LOAN_FEE_BPS as u128 / 10_000u128) as u64
    }

    fn receiver_program_id() -> Result<Pubkey> {
        let program_id = env::var("FLASH_LOAN_RECEIVER_PROGRAM_ID")
            .map_err(|_| anyhow!("FLASH_LOAN_RECEIVER_PROGRAM_ID not set"))?;
        if program_id.trim().is_empty() {
            return Err(anyhow!("FLASH_LOAN_RECEIVER_PROGRAM_ID not set"));
        }
        Pubkey::from_str(&program_id).map_err(|_| anyhow!("invalid FLASH_LOAN_RECEIVER_PROGRAM_ID"))
    }
}

impl fmt::Debug for PortFinanceProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PortFinanceProvider").finish()
    }
}

#[async_trait]
impl FlashLoanProvider for PortFinanceProvider {
    fn id(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn program_id(&self) -> Pubkey {
        PORT_FINANCE_PROGRAM_ID
    }

    fn display_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Port Finance")
    }

    fn supports_token(&self, mint: &Pubkey) -> bool {
        *mint == WSOL_MINT
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

        let capped = request.amount.min(MAX_LOAN_LAMPORTS);
        let fee_lamports = Self::compute_fee_lamports(capped);

        Ok(Some(QuoteResponse {
            provider: PROVIDER_ID,
            token_mint: request.token_mint,
            available_amount: capped,
            fee_bps: FLASH_LOAN_FEE_BPS,
            fee_lamports,
            estimated_latency_ms: 170,
            confidence: 0.5,
        }))
    }

    async fn build_plan(
        &self,
        _environment: &Environment,
        request: &PlanRequest,
    ) -> Result<PlanResponse> {
        if !self.supports_token(&request.token_mint) {
            return Err(anyhow!("unsupported mint for Port Finance flash loan"));
        }

        let receiver_program = Self::receiver_program_id()?;
        let amount = request.amount.min(MAX_LOAN_LAMPORTS);
        let fee = Self::compute_fee_lamports(amount);

        let mut extra_accounts = Vec::new();
        extra_accounts.push(AccountMeta::new_readonly(DEVNET_RESERVE, false));
        extra_accounts.push(AccountMeta::new_readonly(
            DEVNET_RESERVE_LIQUIDITY_SUPPLY,
            false,
        ));
        extra_accounts.push(AccountMeta::new_readonly(DEVNET_FEE_RECEIVER, false));

        let execute_ix = build_execute_flash_loan_ix(
            &receiver_program,
            request.borrower,
            PROVIDER_ID.as_str(),
            request.token_mint,
            amount,
            extra_accounts,
        )?;

        let mut instructions = PlanInstructions::new();
        instructions.borrow.push(execute_ix);

        Ok(PlanResponse {
            provider: PROVIDER_ID,
            amount,
            fee_lamports: fee,
            instructions,
        })
    }
}

fn devnet_config() -> PortFinanceReserveConfig {
    let (market_authority, _) =
        Pubkey::find_program_address(&[DEVNET_LENDING_MARKET.as_ref()], &PORT_FINANCE_PROGRAM_ID);

    PortFinanceReserveConfig {
        program_id: PORT_FINANCE_PROGRAM_ID,
        lending_market: DEVNET_LENDING_MARKET,
        market_authority,
        reserve: DEVNET_RESERVE,
        reserve_liquidity_supply: DEVNET_RESERVE_LIQUIDITY_SUPPLY,
        fee_receiver: DEVNET_FEE_RECEIVER,
        token_program_id: SPL_TOKEN_PROGRAM_ID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::Environment;
    use futures::executor::block_on;
    use solana_sdk::pubkey::Pubkey;
    use std::sync::{Mutex, OnceLock};

    const TEST_BORROWER: Pubkey =
        solana_sdk::pubkey!("8xyqL976aF8CkHSykRGJ4zGHcCh6eqJ9HYU4xLazG2cV");

    fn provider() -> PortFinanceProvider {
        let cfg = devnet_config();
        cfg.validate().expect("valid devnet config");
        PortFinanceProvider::new()
    }

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn set_receiver_env() {
        std::env::set_var(
            "FLASH_LOAN_RECEIVER_PROGRAM_ID",
            "26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg",
        );
    }

    #[test]
    fn build_plan_uses_receiver_program() {
        let _guard = env_guard();
        set_receiver_env();
        let provider = provider();
        let plan = block_on(provider.build_plan(
            &Environment::default(),
            &PlanRequest {
                token_mint: WSOL_MINT,
                amount: 3_000,
                borrower: TEST_BORROWER,
            },
        ))
        .expect("plan builds");

        assert_eq!(plan.provider, PROVIDER_ID);
        assert_eq!(plan.instructions.borrow.len(), 1);
        assert_eq!(
            plan.instructions.borrow[0].program_id,
            Pubkey::from_str("26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg").unwrap()
        );
    }

    #[test]
    fn config_validation_detects_bad_authority() {
        let mut cfg = devnet_config();
        cfg.market_authority = Pubkey::new_unique();
        assert!(cfg.validate().is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::plugin::{Environment, PlanRequest, QuoteRequest};
    use solana_sdk::pubkey::Pubkey;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    #[tokio::test]
    async fn port_finance_quotes_wsol() {
        let _guard = env_guard();
        let provider = PortFinanceProvider::new();
        let quote = provider
            .quote(
                &Environment::default(),
                &QuoteRequest {
                    token_mint: WSOL_MINT,
                    amount: 15_000_000_000,
                },
            )
            .await
            .expect("quote succeeds")
            .expect("quote available");

        assert_eq!(quote.provider, PROVIDER_ID);
        assert_eq!(quote.available_amount, 15_000_000_000);
    }

    #[tokio::test]
    async fn port_finance_plan_uses_receiver_instruction() {
        let _guard = env_guard();
        std::env::set_var(
            "FLASH_LOAN_RECEIVER_PROGRAM_ID",
            "26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg",
        );
        let provider = PortFinanceProvider::new();
        let borrower = Pubkey::new_unique();
        let plan = provider
            .build_plan(
                &Environment::default(),
                &PlanRequest {
                    token_mint: WSOL_MINT,
                    amount: 5_000_000_000,
                    borrower,
                },
            )
            .await
            .expect("plan succeeds");
        assert_eq!(plan.instructions.borrow.len(), 1);
        assert_eq!(
            plan.instructions.borrow[0].program_id,
            Pubkey::from_str("26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg").unwrap()
        );
    }
}
