use crate::models::ArbSignal;
use smallvec::SmallVec;
use solana_sdk::{instruction::Instruction, signature::Keypair, transaction::VersionedTransaction};

use solana_compute_budget_interface::ComputeBudgetInstruction;

// Submit multiple transactions atomically via a block engine (e.g., Jito)
#[async_trait::async_trait]
pub trait BundleSubmitter: Send + Sync {
    async fn submit_bundle(&self, txs: &[VersionedTransaction]) -> anyhow::Result<()>;
}

// Optional flash-loan provider (e.g., Marginfi, etc.). Adapter per protocol.
#[async_trait::async_trait]
pub trait FlashLoanProvider: Send + Sync {
    async fn build_flash_loan_bundle(
        &self,
        amount: u64,
        route_instructions: &[Instruction],
    ) -> anyhow::Result<SmallVec<[Instruction; 16]>>;
}

// Build + sign the final transactions. Keep this pre-warmed: fee payer, compute budget, etc.
#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, sig: &ArbSignal) -> anyhow::Result<()>;
}

// ---- A minimal reference executor (fill in your DEX adapters below) ----

pub struct FastExecutor<S: BundleSubmitter, F: FlashLoanProvider> {
    pub fee_payer: Keypair,
    pub bundler: S,
    pub fl: Option<F>,
}

impl<S: BundleSubmitter, F: FlashLoanProvider> FastExecutor<S, F> {
    pub fn new(fee_payer: Keypair, bundler: S, fl: Option<F>) -> Self {
        Self {
            fee_payer,
            bundler,
            fl,
        }
    }
}

#[async_trait::async_trait]
impl<S: BundleSubmitter + Send + Sync, F: FlashLoanProvider + Send + Sync> Executor
    for FastExecutor<S, F>
{
    async fn execute(&self, sig: &ArbSignal) -> anyhow::Result<()> {
        // 1) Build route instructions for both legs using your DEX adapters
        //    (keep pre-initialized account metas; just fill amounts)
        let leg1: SmallVec<[Instruction; 8]> = build_leg_sell(sig).await?;
        let leg2: SmallVec<[Instruction; 8]> = build_leg_buy(sig).await?;

        // 2) Optionally wrap with flash loan prologue/epilogue
        let mut all_insts: SmallVec<[Instruction; 16]> = SmallVec::new();

        if let Some(fl) = &self.fl {
            let wrapped = fl
                .build_flash_loan_bundle(
                    sig.amount_in as u64,
                    &[leg1.clone(), leg2.clone()].concat(),
                )
                .await?;
            all_insts.extend(wrapped);
        } else {
            all_insts.extend(leg1);
            all_insts.extend(leg2);
        }

        // 3) Add compute budget + priority fees (CU price from knobs)
        let kb = crate::config::get_knobs();
        let (cu_limit, cu_price) = (1_000_000u32, kb.priority_fee_microlamports);
        // OLD (failing):

        let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(cu_limit);
        let price_ix = ComputeBudgetInstruction::set_compute_unit_price(cu_price);
        let mut with_budget: smallvec::SmallVec<[Instruction; 18]> = smallvec::SmallVec::new();
        with_budget.push(budget_ix);
        with_budget.push(price_ix);
        with_budget.extend(all_insts.into_iter());

        // 4) Build VersionedTransaction(s) and submit as a bundle
        let tx = build_payer_tx(&self.fee_payer, &with_budget)?;
        self.bundler.submit_bundle(std::slice::from_ref(&tx)).await
    }
}

// ---- You’ll fill these using your DEX parsers/instruction builders ----
async fn build_leg_sell(_sig: &ArbSignal) -> anyhow::Result<SmallVec<[Instruction; 8]>> {
    Ok(SmallVec::new())
}
async fn build_leg_buy(_sig: &ArbSignal) -> anyhow::Result<SmallVec<[Instruction; 8]>> {
    Ok(SmallVec::new())
}

// Construct a VersionedTransaction (stub: attach recent blockhash from a cached feed)
fn build_payer_tx(
    _payer: &Keypair,
    _insts: &[Instruction],
) -> anyhow::Result<VersionedTransaction> {
    // Use a shared “recent blockhash” cache fed by your geyser/WS client to avoid RPC calls.
    anyhow::bail!("wire this to your blockhash cache + message builder")
}
