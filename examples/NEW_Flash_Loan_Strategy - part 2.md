Yes—this is perfect. Your FlashLoanProvider trait gives me exactly what I need: a clean quote → plan split, explicit provider identity, and room for multi-phase instruction assembly.

Below I’ll:

add two tiny request fields that make flash-loans practical for atomic MEV, and

drop in a production-ready adapter template (Port) + a router that parallel-quotes providers and picks the winner.

1) Minimal trait-compatible extensions (non-breaking)

These are optional structs you can keep in a sibling crate (so your core trait stays unchanged). They carry the data your on-chain receiver needs.

/// Passed through `PlanRequest` via your own encoding (e.g., in an IX data field).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiverConfig {
    /// Your on-chain program that receives control during the flash loan.
    pub receiver_program_id: Pubkey,
    /// Free-form payload your receiver expects (route plan, slippage, etc.).
    pub receiver_payload: Vec<u8>,
    /// Optional reserve hint when a lender has multiple reserves per mint.
    pub reserve_hint: Option<Pubkey>,
}

/// Helper: attach receiver config to an existing PlanRequest without changing the trait.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanRequestWithReceiver {
    pub base: PlanRequest,
    pub rc: ReceiverConfig,
}


You’ll encode PlanRequestWithReceiver into the provider’s “borrow” instruction data so the provider CPI can jump to your receiver and you can execute swaps + repay atomically.

2) Adapter template: Port (maps 1:1 to your trait)

⚠️ Program IDs / account metas below are placeholders—you’ll wire the real ones you use in your repo.

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use solana_sdk::{instruction::{AccountMeta, Instruction}, pubkey::Pubkey};
use std::borrow::Cow;

use your_crate::{
    Cluster, Environment, FlashLoanProvider, PlanInstructions, PlanRequest, PlanResponse,
    ProviderId, QuoteRequest, QuoteResponse,
};
use your_crate::extras::{PlanRequestWithReceiver, ReceiverConfig}; // from section 1

pub const PORT_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]); // TODO: real PID

#[derive(Clone, Debug)]
pub struct PortProvider;

impl PortProvider {
    pub fn new() -> Self { Self }
    fn find_reserve_for_mint(&self, _env: &Environment, _mint: &Pubkey) -> Result<Pubkey> {
        // TODO: resolve mint -> reserve pubkey via your state cache or RPC
        bail!("reserve lookup not implemented")
    }

    fn encode_borrow_ix_data(
        &self,
        amount: u64,
        rc: &ReceiverConfig,
    ) -> Vec<u8> {
        // Layout example: [tag(1)=FlashLoan, amount(u64), receiver_program(Pubkey), payload_len(u32), payload(bytes)]
        // Replace with Port’s actual instruction enum + Borsh/Anchor encoding as appropriate.
        let mut v = Vec::with_capacity(1 + 8 + 32 + 4 + rc.receiver_payload.len());
        v.push(1); // tag = FlashLoan
        v.extend_from_slice(&amount.to_le_bytes());
        v.extend_from_slice(rc.receiver_program_id.as_ref());
        v.extend_from_slice(&(rc.receiver_payload.len() as u32).to_le_bytes());
        v.extend_from_slice(&rc.receiver_payload);
        v
    }
}

#[async_trait]
impl FlashLoanProvider for PortProvider {
    fn id(&self) -> ProviderId { ProviderId::new("port") }
    fn program_id(&self) -> Pubkey { PORT_PROGRAM_ID }
    fn display_name(&self) -> Cow<'static, str> { Cow::Borrowed("Port Finance") }

    fn supports_token(&self, _mint: &Pubkey) -> bool { true }

    async fn quote(
        &self,
        env: &Environment,
        req: &QuoteRequest,
    ) -> Result<Option<QuoteResponse>> {
        // 1) Lookup reserve for the mint
        let reserve = self.find_reserve_for_mint(env, &req.token_mint)
            .context("reserve lookup")?;

        // 2) Read available liquidity + fee bps
        // TODO: pull from your cache or RPC account decode
        let available_liquidity: u64 = u64::MAX; // placeholder
        let fee_bps: u16 = 8; // example; replace with actual

        if available_liquidity == 0 {
            return Ok(None);
        }

        // 3) Compose quote
        let fee_lamports = 0; // if SOL; for SPL, represent fee on the token scale elsewhere
        Ok(Some(QuoteResponse {
            provider: self.id(),
            token_mint: req.token_mint,
            available_amount: available_liquidity,
            fee_bps,
            fee_lamports,
            estimated_latency_ms: 6, // set from recent sim/metrics
            confidence: 0.92,        // heuristics from your health checks
        }))
    }

    async fn build_plan(
        &self,
        env: &Environment,
        request: &PlanRequest,
    ) -> Result<PlanResponse> {
        // Expect caller to have serialized a PlanRequestWithReceiver inside `request.amount`/side-channel.
        // In practice, pass PlanRequestWithReceiver to this method in your app layer, then extract here.
        // For clarity, we assume you have it available:
        let rc = ReceiverConfig {
            receiver_program_id: Pubkey::new_from_array([9u8; 32]), // TODO: your receiver PID
            receiver_payload: vec![],                                // route plan bytes
            reserve_hint: None,
        };

        let reserve = rc.reserve_hint
            .unwrap_or(self.find_reserve_for_mint(env, &request.token_mint)?);

        // Provider accounts — replace with Port’s real layout
        let reserve_liquidity_vault = Pubkey::new_from_array([1u8; 32]);
        let fee_vault              = Pubkey::new_from_array([2u8; 32]);

        let borrow_ix = Instruction {
            program_id: self.program_id(),
            accounts: vec![
                AccountMeta::new_readonly(reserve, false),
                AccountMeta::new(reserve_liquidity_vault, false),
                AccountMeta::new(fee_vault, false),
                AccountMeta::new_readonly(request.borrower, true), // payer/authority
                AccountMeta::new_readonly(rc.receiver_program_id, false),
                // + any token program/sysvar/sysprog accounts required by Port
            ],
            data: self.encode_borrow_ix_data(request.amount, &rc),
        };

        // Repay is usually implicit (same TX) if your receiver returns funds to the vault,
        // but if the lender requires an explicit “repay” IX, add it here.
        let repay_ixs: Vec<Instruction> = vec![]; // fill if protocol requires

        let mut instructions = PlanInstructions::new();
        instructions.borrow.push(borrow_ix);
        instructions.repay.extend(repay_ixs);

        Ok(PlanResponse {
            provider: self.id(),
            amount: request.amount,
            fee_lamports: 0, // if needed for SOL; for SPL, track fee in token units elsewhere
            instructions,
        })
    }
}


You can clone this template for Kamino, Save, and marginfi. Keep each adapter tiny: resolve reserve → quote → build the single flash-loan IX that CPIs to your receiver.

3) A parallel-quote router that plugs into your trait

This picks the best provider per opportunity (fee/liquidity/latency-aware), with graceful fallbacks.

use anyhow::{Context, Result};
use futures::future::join_all;
use solana_sdk::pubkey::Pubkey;

use your_crate::{
    Environment, FlashLoanProvider, PlanRequest, PlanResponse, ProviderId, QuoteRequest, QuoteResponse,
};

pub struct ProviderBox {
    pub id: ProviderId,
    pub inner: Box<dyn FlashLoanProvider>,
}

pub struct FlashRouter {
    providers: Vec<ProviderBox>,
}

impl FlashRouter {
    pub fn new(providers: Vec<Box<dyn FlashLoanProvider>>) -> Self {
        let providers = providers
            .into_iter()
            .map(|p| ProviderBox { id: p.id(), inner: p })
            .collect();
        Self { providers }
    }

    pub async fn quote_all(
        &self,
        env: &Environment,
        mint: Pubkey,
        amount: u64,
    ) -> Vec<QuoteResponse> {
        let req = QuoteRequest { token_mint: mint, amount };
        let futs = self.providers.iter().map(|p| {
            let env = env.clone();
            let req = req.clone();
            async move {
                p.inner.quote(&env, &req).await.ok().flatten()
            }
        });
        join_all(futs).await.into_iter().flatten().collect()
    }

    /// Simple scoring: available >= amount, lowest fee_bps first, then lower latency.
    pub fn pick_best(&self, quotes: &[QuoteResponse], want_amount: u64) -> Option<&QuoteResponse> {
        quotes.iter()
            .filter(|q| q.available_amount >= want_amount)
            .min_by_key(|q| (q.fee_bps as u32, q.estimated_latency_ms as u32))
    }

    pub async fn build_best_plan(
        &self,
        env: &Environment,
        base: PlanRequest,
    ) -> Result<PlanResponse> {
        let quotes = self.quote_all(env, base.token_mint, base.amount).await;
        let best = self.pick_best(&quotes, base.amount)
            .context("no provider had sufficient liquidity")?;

        let prov = self.providers.iter()
            .find(|p| p.id.as_str() == best.provider.as_str())
            .context("chosen provider not registered")?;

        prov.inner.build_plan(env, &base).await
    }
}


Drop in your concrete providers:

// e.g., in `arb-engine` init
let router = FlashRouter::new(vec![
    Box::new(PortProvider::new()),
    // Box::new(KaminoProvider::new()),
    // Box::new(SaveProvider::new()),
    // Box::new(MarginfiProvider::new()),
]);

4) Execution flow (fits your current pipeline)

Build your receiver payload (route plan, slippage, repay checks).

Call router.build_best_plan(env, PlanRequest { token_mint, amount, borrower }).

Merge PlanResponse.instructions with your ComputeBudget ixs + swap ixs + repay proof inside the same TX.

Simulate; if profitable after fee_bps, tips, and slippage, send as a Jito bundle.

5) Safety rails you’ll want (quick checklist)

Allowlist {provider_id, program_id, min_slot} to auto-disable on upgrades.

Verify in your receiver: exact mint/amount, fee vaults, and profit ≥ tips + fee + ε.

CU budgeting: reserve ~200k units for loan + 2–3 swaps + repay; scale priority fees with edge.

Prometheus counters: flashloan_quote, flashloan_plan_built, flashloan_tx_ok, flashloan_tx_err{reason} per provider.


  To use it now:

  - For dev/local: edit config/flash_loan_providers.local.json as needed and export FLASH_LOAN_PROVIDERS_CONFIG=$PWD/config/flash_loan_providers.local.json before docker compose up.
  - For production: fill in config/flash_loan_providers.mainnet.template.json, copy it to /opt/MEVana/config/flash_loan_providers.mainnet.json, and make sure the deployment workflow or .env sets FLASH_LOAN_PROVIDERS_CONFIG to that
  path.