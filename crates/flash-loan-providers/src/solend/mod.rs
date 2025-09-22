mod instructions;

use self::instructions::{
    create_flash_borrow_instruction, create_flash_repay_instruction, SolendReserveConfig,
};
use crate::plugin::{
    Environment, FlashLoanProvider, PlanInstructions, PlanRequest, PlanResponse, ProviderId,
    QuoteRequest, QuoteResponse,
};
use crate::registry::register_provider;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{info, warn};
use solana_sdk::pubkey::Pubkey;
use std::borrow::Cow;
use std::env;
use std::fmt;
use std::sync::Arc;

const WSOL_MINT: Pubkey = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");

const DEVNET_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ALend7Ketfx5bxh6ghsCDXAoDrhvEmsXT3cynB6aPLgx");
const DEVNET_LENDING_MARKET: Pubkey =
    solana_sdk::pubkey!("GvjoVKNjBvQcFaSKUW1gTE7DxhSpjHbE69umVR5nPuQp");
const DEVNET_MARKET_AUTHORITY: Pubkey =
    solana_sdk::pubkey!("EhJ4fwaXUp7aiwvZThSUaGWCaBQAJe3AEaJJJVCn3UCK");
const DEVNET_RESERVE: Pubkey = solana_sdk::pubkey!("5VVLD7BQp8y3bTgyF5ezm1ResyMTR3PhYsT4iHFU8Sxz");
const DEVNET_RESERVE_LIQUIDITY_SUPPLY: Pubkey =
    solana_sdk::pubkey!("furd3XUtjXZ2gRvSsoUts9A5m8cMJNqdsyR2Rt8vY9s");
const DEVNET_FEE_RECEIVER: Pubkey =
    solana_sdk::pubkey!("5kFqzU2k1tEXtoeNayk1TVxLycoAH5k8WsaGnBnanYJH");

// Host fee receiver can be overridden via `SOLEND_WSOL_HOST_FEE_ACCOUNT`.
const DEVNET_HOST_FEE_FALLBACK: Pubkey = DEVNET_FEE_RECEIVER;

const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

const MAX_LOAN_LAMPORTS: u64 = 50_000_000_000; // 50 SOL cap until live data is queried
const FLASH_LOAN_FEE_BPS: u16 = 30; // 0.30%

pub const PROVIDER_ID: ProviderId = ProviderId::new("solend");

#[derive(Clone)]
pub struct SolendProvider {
    config: Arc<SolendReserveConfig>,
}

fn associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), SPL_TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

impl SolendProvider {
    pub fn register() {
        match Self::load_devnet_config() {
            Ok(config) => {
                if let Err(err) = config.validate() {
                    warn!("Solend config validation failed: {err:?}");
                    return;
                }
                let provider = Arc::new(Self {
                    config: Arc::new(config),
                });
                if register_provider(provider) {
                    info!("Registered Solend flash loan provider (WSOL devnet)");
                }
            }
            Err(err) => warn!("Failed to register Solend provider: {err:?}"),
        }
    }

    fn load_devnet_config() -> Result<SolendReserveConfig> {
        let host_fee = env::var("SOLEND_WSOL_HOST_FEE_ACCOUNT")
            .ok()
            .and_then(|value| value.parse::<Pubkey>().ok())
            .unwrap_or_else(|| {
                warn!(
                    "SOLEND_WSOL_HOST_FEE_ACCOUNT not set – host fees will accrue to the reserve fee receiver"
                );
                DEVNET_HOST_FEE_FALLBACK
            });

        Ok(SolendReserveConfig {
            program_id: DEVNET_PROGRAM_ID,
            lending_market: DEVNET_LENDING_MARKET,
            reserve: DEVNET_RESERVE,
            reserve_liquidity_supply: DEVNET_RESERVE_LIQUIDITY_SUPPLY,
            lending_market_authority: DEVNET_MARKET_AUTHORITY,
            fee_receiver: DEVNET_FEE_RECEIVER,
            host_fee_receiver: host_fee,
            token_program_id: SPL_TOKEN_PROGRAM_ID,
        })
    }

    fn compute_fee_lamports(&self, amount: u64) -> u64 {
        let total_fee = (amount as u128 * FLASH_LOAN_FEE_BPS as u128) / 10_000u128;
        total_fee as u64
    }
}

impl fmt::Debug for SolendProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SolendProvider")
            .field("reserve", &self.config.reserve)
            .finish()
    }
}

#[async_trait]
impl FlashLoanProvider for SolendProvider {
    fn id(&self) -> ProviderId {
        PROVIDER_ID
    }

    fn program_id(&self) -> Pubkey {
        self.config.program_id
    }

    fn display_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Solend")
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

        let capped_amount = request.amount.min(MAX_LOAN_LAMPORTS);
        let fee_lamports = self.compute_fee_lamports(capped_amount);

        Ok(Some(QuoteResponse {
            provider: PROVIDER_ID,
            token_mint: request.token_mint,
            available_amount: capped_amount,
            fee_bps: FLASH_LOAN_FEE_BPS,
            fee_lamports,
            estimated_latency_ms: 140,
            confidence: 0.55,
        }))
    }

    async fn build_plan(
        &self,
        _environment: &Environment,
        request: &PlanRequest,
    ) -> Result<PlanResponse> {
        if !self.supports_token(&request.token_mint) {
            return Err(anyhow!("unsupported mint for Solend flash loan"));
        }

        let amount = request.amount.min(MAX_LOAN_LAMPORTS);
        if amount == 0 {
            return Err(anyhow!("flash loan amount must be greater than zero"));
        }

        let borrower_token_account =
            associated_token_address(&request.borrower, &request.token_mint);

        let borrow_ix =
            create_flash_borrow_instruction(&self.config, borrower_token_account, amount);
        let repay_ix = create_flash_repay_instruction(
            &self.config,
            borrower_token_account,
            request.borrower,
            amount,
            0,
        );

        let mut instructions = PlanInstructions::new();
        instructions.borrow.push(borrow_ix);
        instructions.repay.push(repay_ix);

        let fee_lamports = self.compute_fee_lamports(amount);

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
    use crate::plugin::{Environment, PlanRequest, QuoteRequest};
    use crate::registry::{clear_registry, register_provider};
    use solana_sdk::{pubkey::Pubkey, sysvar};
    use std::convert::TryInto;
    use std::str::FromStr;
    use std::sync::Arc;

    const TEST_BORROWER: Pubkey =
        solana_sdk::pubkey!("9NuxsZyJLPJEMwSfj2nU5sYmqL1ZUCLjtTnpJc7Aa1gE");
    const TEST_HOST_FEE: &str = "AJVS4MHtdmwnKAkKbuX7FUKVwhuEM1woVRVctS4Gi33S";

    fn provider() -> SolendProvider {
        std::env::set_var("SOLEND_WSOL_HOST_FEE_ACCOUNT", TEST_HOST_FEE);
        let cfg = SolendProvider::load_devnet_config().expect("devnet config");
        SolendProvider {
            config: Arc::new(cfg),
        }
    }

    #[test]
    fn quote_and_plan_for_wsol() {
        clear_registry();
        let provider = provider();
        register_provider(Arc::new(provider.clone()));

        let quote = futures::executor::block_on(provider.quote(
            &Environment::default(),
            &QuoteRequest {
                token_mint: WSOL_MINT,
                amount: 25_000_000_000,
            },
        ))
        .expect("quote ok")
        .expect("available quote");

        assert_eq!(quote.provider, PROVIDER_ID);
        assert_eq!(quote.token_mint, WSOL_MINT);
        assert_eq!(quote.available_amount, 25_000_000_000);
        assert_eq!(
            quote.fee_lamports,
            provider.compute_fee_lamports(25_000_000_000)
        );

        let plan = futures::executor::block_on(provider.build_plan(
            &Environment::default(),
            &PlanRequest {
                token_mint: WSOL_MINT,
                amount: 10_000_000_000,
                borrower: TEST_BORROWER,
            },
        ))
        .expect("plan builds");

        assert_eq!(plan.provider, PROVIDER_ID);
        assert_eq!(plan.amount, 10_000_000_000);
        assert_eq!(plan.instructions.borrow.len(), 1);
        assert_eq!(plan.instructions.repay.len(), 1);
        assert!(plan.instructions.setup.is_empty());
        assert!(plan.instructions.cleanup.is_empty());
        assert!(plan.fee_lamports > 0);
    }

    #[test]
    fn build_plan_wires_borrow_and_repay_correctly() {
        let provider = provider();
        let env = Environment::default();
        let amount = 5_000_000_000u64; // 5 SOL

        let response = futures::executor::block_on(provider.build_plan(
            &env,
            &PlanRequest {
                token_mint: WSOL_MINT,
                amount,
                borrower: TEST_BORROWER,
            },
        ))
        .expect("plan response");

        assert_eq!(response.provider, PROVIDER_ID);
        assert_eq!(response.amount, amount);
        assert_eq!(response.fee_lamports, provider.compute_fee_lamports(amount));

        let borrower_ata = associated_token_address(&TEST_BORROWER, &WSOL_MINT);

        let borrow_ix = &response.instructions.borrow[0];
        assert_eq!(borrow_ix.program_id, DEVNET_PROGRAM_ID);
        assert_eq!(borrow_ix.accounts.len(), 7);
        assert_eq!(
            borrow_ix.accounts[0].pubkey,
            DEVNET_RESERVE_LIQUIDITY_SUPPLY
        );
        assert_eq!(borrow_ix.accounts[1].pubkey, borrower_ata);
        assert_eq!(borrow_ix.accounts[2].pubkey, DEVNET_RESERVE);
        assert_eq!(borrow_ix.accounts[3].pubkey, DEVNET_LENDING_MARKET);
        assert_eq!(borrow_ix.accounts[4].pubkey, DEVNET_MARKET_AUTHORITY);
        assert_eq!(borrow_ix.accounts[6].pubkey, SPL_TOKEN_PROGRAM_ID);
        assert_eq!(borrow_ix.data[0], 19);
        assert_eq!(
            u64::from_le_bytes(borrow_ix.data[1..9].try_into().unwrap()),
            amount
        );

        let repay_ix = &response.instructions.repay[0];
        assert_eq!(repay_ix.program_id, DEVNET_PROGRAM_ID);
        assert_eq!(repay_ix.accounts.len(), 9);
        assert_eq!(repay_ix.accounts[0].pubkey, borrower_ata);
        assert_eq!(repay_ix.accounts[1].pubkey, DEVNET_RESERVE_LIQUIDITY_SUPPLY);
        assert_eq!(repay_ix.accounts[2].pubkey, DEVNET_FEE_RECEIVER);
        let expected_host = Pubkey::from_str(TEST_HOST_FEE).expect("host fee pubkey");
        assert_eq!(repay_ix.accounts[3].pubkey, expected_host);
        assert_eq!(repay_ix.accounts[4].pubkey, DEVNET_RESERVE);
        assert_eq!(repay_ix.accounts[5].pubkey, DEVNET_LENDING_MARKET);
        assert_eq!(repay_ix.accounts[6].pubkey, TEST_BORROWER);
        assert_eq!(repay_ix.accounts[7].pubkey, sysvar::instructions::id());
        assert_eq!(repay_ix.accounts[8].pubkey, SPL_TOKEN_PROGRAM_ID);
        assert_eq!(repay_ix.data[0], 20);
        assert_eq!(
            u64::from_le_bytes(repay_ix.data[1..9].try_into().unwrap()),
            amount
        );
        assert_eq!(repay_ix.data[9], 0);
    }

    #[test]
    fn reject_wrong_mint() {
        let provider = provider();
        let quote = futures::executor::block_on(provider.quote(
            &Environment::default(),
            &QuoteRequest {
                token_mint: Pubkey::new_unique(),
                amount: 1_000,
            },
        ))
        .expect("quote ok");
        assert!(quote.is_none());
    }

    #[test]
    fn config_validation_rejects_bad_authority() {
        let mut cfg = SolendProvider::load_devnet_config().expect("devnet config");
        cfg.lending_market_authority = Pubkey::new_unique();
        assert!(cfg.validate().is_err());
    }
}
