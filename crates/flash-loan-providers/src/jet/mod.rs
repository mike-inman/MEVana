use crate::plugin::{
    Environment, FlashLoanProvider, PlanInstructions, PlanRequest, PlanResponse, ProviderId,
    QuoteRequest, QuoteResponse,
};
use crate::registry::register_provider;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use flash_loan_receiver::build_execute_flash_loan_ix;
use log::{info, warn};
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};
use std::borrow::Cow;
use std::env;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

const JET_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("JPLockxtkngHkaQT5AuRYow3HyUv5qWzmhwsCPd653n");
const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const FLASH_LOAN_FEE_BPS: u16 = 35; // Placeholder until fee data is fetched dynamically

pub const PROVIDER_ID: ProviderId = ProviderId::new("jetprotocol");

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct JetReserveConfig {
    program_id: Pubkey,
    lending_market: Pubkey,
    market_authority: Pubkey,
    reserve: Pubkey,
    reserve_liquidity_supply: Pubkey,
    fee_receiver: Pubkey,
    token_program_id: Pubkey,
}

impl JetReserveConfig {
    fn validate(&self) -> Result<()> {
        let (derived_authority, _) =
            Pubkey::find_program_address(&[self.lending_market.as_ref()], &self.program_id);
        if derived_authority != self.market_authority {
            return Err(anyhow!(
                "jet config mismatch: derived authority {} != provided {}",
                derived_authority,
                self.market_authority
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct JetProvider;

impl JetProvider {
    fn new() -> Self {
        Self
    }

    pub fn register() {
        match load_config_from_env() {
            Ok(config) => {
                if let Err(err) = config.validate() {
                    warn!("Skipping Jet provider registration: {err:?}");
                    return;
                }
                let provider = Arc::new(Self::new());
                if register_provider(provider) {
                    info!("Registered Jet flash loan provider (environment configured)");
                }
            }
            Err(err) => {
                warn!(
                    "Jet flash loan provider not registered: {err}. Set JET_LENDING_MARKET, JET_RESERVE, JET_RESERVE_LIQUIDITY_SUPPLY, and JET_FEE_RECEIVER to enable"
                );
            }
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

impl fmt::Debug for JetProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JetProvider").finish()
    }
}

#[async_trait]
impl FlashLoanProvider for JetProvider {
    fn id(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn program_id(&self) -> Pubkey {
        JET_PROGRAM_ID
    }

    fn display_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Jet Protocol")
    }

    fn supports_token(&self, _mint: &Pubkey) -> bool {
        true
    }

    fn max_loan_amount(&self) -> Option<u64> {
        None
    }

    async fn quote(
        &self,
        _environment: &Environment,
        request: &QuoteRequest,
    ) -> Result<Option<QuoteResponse>> {
        let fee_lamports = Self::compute_fee_lamports(request.amount);
        Ok(Some(QuoteResponse {
            provider: PROVIDER_ID,
            token_mint: request.token_mint,
            available_amount: request.amount,
            fee_bps: FLASH_LOAN_FEE_BPS,
            fee_lamports,
            estimated_latency_ms: 180,
            confidence: 0.4,
        }))
    }

    async fn build_plan(
        &self,
        _environment: &Environment,
        request: &PlanRequest,
    ) -> Result<PlanResponse> {
        let config = load_config_from_env()?;
        let receiver_program = Self::receiver_program_id()?;

        let amount = request.amount;
        let fee = Self::compute_fee_lamports(amount);

        let mut extra_accounts = Vec::new();
        extra_accounts.push(AccountMeta::new_readonly(config.lending_market, false));
        extra_accounts.push(AccountMeta::new_readonly(config.reserve, false));
        extra_accounts.push(AccountMeta::new_readonly(
            config.reserve_liquidity_supply,
            false,
        ));
        extra_accounts.push(AccountMeta::new_readonly(config.fee_receiver, false));

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

fn load_config_from_env() -> Result<JetReserveConfig> {
    let lending_market = parse_env_pubkey("JET_LENDING_MARKET")?;
    let reserve = parse_env_pubkey("JET_RESERVE")?;
    let reserve_liquidity_supply = parse_env_pubkey("JET_RESERVE_LIQUIDITY_SUPPLY")?;
    let fee_receiver = parse_env_pubkey("JET_FEE_RECEIVER")?;

    let market_authority = match env::var("JET_MARKET_AUTHORITY") {
        Ok(value) => Pubkey::from_str(&value)
            .map_err(|_| anyhow!("invalid pubkey in JET_MARKET_AUTHORITY: {}", value))?,
        Err(_) => Pubkey::find_program_address(&[lending_market.as_ref()], &JET_PROGRAM_ID).0,
    };

    Ok(JetReserveConfig {
        program_id: JET_PROGRAM_ID,
        lending_market,
        market_authority,
        reserve,
        reserve_liquidity_supply,
        fee_receiver,
        token_program_id: SPL_TOKEN_PROGRAM_ID,
    })
}

fn parse_env_pubkey(var: &str) -> Result<Pubkey> {
    let value = env::var(var).map_err(|_| anyhow!("{} not set", var))?;
    Pubkey::from_str(&value).map_err(|_| anyhow!("invalid pubkey in {}: {}", var, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::Environment;
    use futures::executor::block_on;
    use std::sync::{Mutex, OnceLock};

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
    fn config_validation_detects_bad_authority() {
        let lending_market = Pubkey::new_unique();
        let (market_authority, _) =
            Pubkey::find_program_address(&[lending_market.as_ref()], &JET_PROGRAM_ID);
        let mut cfg = JetReserveConfig {
            program_id: JET_PROGRAM_ID,
            lending_market,
            market_authority,
            reserve: Pubkey::new_unique(),
            reserve_liquidity_supply: Pubkey::new_unique(),
            fee_receiver: Pubkey::new_unique(),
            token_program_id: SPL_TOKEN_PROGRAM_ID,
        };
        cfg.market_authority = Pubkey::new_unique();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn build_plan_uses_receiver_instruction() {
        let _guard = env_guard();
        set_receiver_env();
        std::env::set_var("JET_LENDING_MARKET", Pubkey::new_unique().to_string());
        std::env::set_var("JET_RESERVE", Pubkey::new_unique().to_string());
        std::env::set_var(
            "JET_RESERVE_LIQUIDITY_SUPPLY",
            Pubkey::new_unique().to_string(),
        );
        std::env::set_var("JET_FEE_RECEIVER", Pubkey::new_unique().to_string());

        let provider = JetProvider::new();
        let borrower = Pubkey::new_unique();
        let plan = block_on(provider.build_plan(
            &Environment::default(),
            &PlanRequest {
                token_mint: Pubkey::new_unique(),
                amount: 1_000,
                borrower,
            },
        ))
        .expect("plan builds");

        assert_eq!(plan.instructions.borrow.len(), 1);
        assert_eq!(
            plan.instructions.borrow[0].program_id,
            Pubkey::from_str("26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg").unwrap()
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::plugin::{Environment, PlanRequest, QuoteRequest};
    use crate::registry::{clear_registry, providers, register_provider};
    use solana_sdk::pubkey::Pubkey;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn set_env() {
        std::env::set_var("JET_LENDING_MARKET", Pubkey::new_unique().to_string());
        std::env::set_var("JET_RESERVE", Pubkey::new_unique().to_string());
        std::env::set_var(
            "JET_RESERVE_LIQUIDITY_SUPPLY",
            Pubkey::new_unique().to_string(),
        );
        std::env::set_var("JET_FEE_RECEIVER", Pubkey::new_unique().to_string());
        std::env::remove_var("JET_MARKET_AUTHORITY");
        std::env::set_var(
            "FLASH_LOAN_RECEIVER_PROGRAM_ID",
            "26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg",
        );
    }

    #[tokio::test]
    async fn jet_registers_when_env_is_present() {
        let _guard = env_guard();
        clear_registry();
        set_env();
        JetProvider::register();
        let names: Vec<_> = providers()
            .into_iter()
            .map(|p| p.id().as_str().to_string())
            .collect();
        assert!(names.contains(&PROVIDER_ID.as_str().to_string()));
    }

    #[tokio::test]
    async fn jet_quotes_and_plan_with_receiver_program() {
        let _guard = env_guard();
        clear_registry();
        set_env();
        let provider = Arc::new(JetProvider::new());
        register_provider(provider.clone());
        let quote = provider
            .quote(
                &Environment::default(),
                &QuoteRequest {
                    token_mint: Pubkey::new_unique(),
                    amount: 5_000_000_000,
                },
            )
            .await
            .expect("quote ok")
            .expect("quote present");
        assert_eq!(quote.provider, PROVIDER_ID);

        let plan = provider
            .build_plan(
                &Environment::default(),
                &PlanRequest {
                    token_mint: Pubkey::new_unique(),
                    amount: 5_000_000_000,
                    borrower: Pubkey::new_unique(),
                },
            )
            .await
            .expect("plan builds");
        assert_eq!(plan.instructions.borrow.len(), 1);
        assert_eq!(
            plan.instructions.borrow[0].program_id,
            Pubkey::from_str("26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg").unwrap()
        );
    }
}
