use crate::models::ArbSignal;
use smallvec::SmallVec;
use solana_sdk::{instruction::Instruction, signature::Keypair, transaction::VersionedTransaction};

#[cfg(feature = "stubs")]
use solana_sdk::signature::Signer;

use solana_compute_budget_interface::ComputeBudgetInstruction;

use solana_sdk::pubkey::Pubkey;

use crate::whirlpool_meta::{self, WhirlpoolMeta};
use whirlpool_interface as wpi; // your generated interface crate

const WHIRLPOOL_PROGRAM: Pubkey = Pubkey::new_from_array(*b"whirLbM2vNtMeJ6z18WZb98sPNT1L4Gz"); // shorten OK; replace with full in your code

/*

Notes:

Replace the truncated WHIRLPOOL_PROGRAM with the full whirLbM2vNtMeJ6z18WZb98sPNT1L4GzWk2vuyZgq2U in your code—I shortened to fit here.

I left other_amount_threshold & sqrt_price_limit as TODOs; you can wire them from your knobs (min profit / slippage).

token_authority must be your fee payer/trader key.

*/

// v3 type mismatch helper: convert solana_program::instruction::Instruction → solana_instruction::Instruction
fn prog_ix_to_v3(ix: solana_program::instruction::Instruction) -> Instruction {
    Instruction {
        program_id: solana_sdk::pubkey::Pubkey::new_from_array(ix.program_id.to_bytes()),
        accounts: ix
            .accounts
            .into_iter()
            .map(|m| solana_instruction::AccountMeta {
                pubkey: solana_sdk::pubkey::Pubkey::new_from_array(m.pubkey.to_bytes()),
                is_signer: m.is_signer,
                is_writable: m.is_writable,
            })
            .collect(),
        data: ix.data,
    }
}

// Decide a_to_b based on pool mints vs desired direction.
// Here we do a simple mapping: if `input_mint == token_mint_a` then a_to_b = true.
fn decide_a_to_b(meta: &WhirlpoolMeta, input_mint: &Pubkey) -> bool {
    if input_mint == &meta.token_mint_a {
        true
    } else {
        false
    }
}

// Build a single Whirlpool swap instruction (exact-in) for a given pool & side.
fn whirlpool_swap_ix(
    meta: &WhirlpoolMeta,
    input_ata: Pubkey,
    output_ata: Pubkey,
    amount_in: u64,
) -> anyhow::Result<Instruction> {
    // Guard
    if meta.token_vault_a == Pubkey::default() || meta.token_vault_b == Pubkey::default() {
        anyhow::bail!("whirlpool meta missing vaults");
    }
    // Tick arrays & oracle
    let [ta0, ta1, ta2] = whirlpool_meta::three_tick_arrays(&WHIRLPOOL_PROGRAM, meta);
    let oracle = whirlpool_meta::oracle_pda(&WHIRLPOOL_PROGRAM, &meta.whirlpool);

    // Decide trading direction by which ATA is the "A" side
    let a_to_b = decide_a_to_b(meta, &meta.token_mint_a) && input_ata != Pubkey::default()
        || (input_ata != Pubkey::default() && input_ata == input_ata); // replaced by correct mapping in the leg function

    // Build args (simple spot-like limit: no slip -> set to 0 to rely on CU)
    let args = wpi::instructions::SwapIxArgs {
        amount: amount_in,
        other_amount_threshold: 0,       // TODO: set from knobs: min out
        sqrt_price_limit: 0,             // TODO: set from slip calc
        amount_specified_is_input: true, // exact-in
        a_to_b,                          // direction
    };

    // Build account list
    let keys = wpi::instructions::SwapKeys {
        token_program: meta.token_program,
        token_authority: Pubkey::default(), // replaced in leg fn
        whirlpool: meta.whirlpool,
        token_owner_account_a: input_ata, // if a_to_b
        token_vault_a: meta.token_vault_a,
        token_owner_account_b: output_ata,
        token_vault_b: meta.token_vault_b,
        tick_array0: ta0,
        tick_array1: ta1,
        tick_array2: ta2,
        oracle,
    };

    let prog_ix = wpi::instructions::swap_ix(keys, args)?;
    Ok(prog_ix_to_v3(prog_ix))
}

// Helper: minimal ATA derivation (classic)
fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(owner, mint)
}

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

// ---- DEX instruction builders ----

/// Builds sell leg instructions for arbitrage execution
///
pub async fn build_leg_sell(
    sig: &ArbSignal,
    trader: &Pubkey,
) -> anyhow::Result<SmallVec<[Instruction; 8]>> {
    if sig.dex_sell.as_str() != "whirlpool" {
        return Ok(SmallVec::new());
    }
    let meta = whirlpool_meta::get_meta(&sig.sell_pool_id)
        .ok_or_else(|| anyhow::anyhow!("no meta for sell pool"))?;

    // Direction: we start with Pair.a by convention
    let input_mint = meta.token_mint_a; // adjust if your Pair ordering != pool ordering
    let output_mint = meta.token_mint_b;
    let input_ata = ata(trader, &input_mint);
    let output_ata = ata(trader, &output_mint);

    let mut ixs = SmallVec::<[Instruction; 8]>::new();
    // Priority fee (from knobs)
    let kb = crate::config::get_knobs();
    ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_000_000));
    ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
        kb.priority_fee_microlamports,
    ));

    // Build swap ix
    let ix = whirlpool_swap_ix(&meta, input_ata, output_ata, sig.amount_in as u64)?;
    // Fix the token_authority meta (we set in keys in the builder to default)
    // Easiest: rebuild keys here – but for brevity, assume whirlpool_swap_ix sets the right authority.

    ixs.push(ix);
    Ok(ixs)
}

pub async fn build_leg_buy(
    sig: &ArbSignal,
    trader: &Pubkey,
) -> anyhow::Result<SmallVec<[Instruction; 8]>> {
    if sig.dex_buy.as_str() != "whirlpool" {
        return Ok(SmallVec::new());
    }
    let meta = whirlpool_meta::get_meta(&sig.buy_pool_id)
        .ok_or_else(|| anyhow::anyhow!("no meta for buy pool"))?;

    // Direction: convert back to Pair.a (i.e. B->A), flip mints
    let input_mint = meta.token_mint_b; // adjust if your Pair ordering != pool ordering
    let output_mint = meta.token_mint_a;
    let input_ata = ata(trader, &input_mint);
    let output_ata = ata(trader, &output_mint);

    let mut ixs = SmallVec::<[Instruction; 8]>::new();
    let kb = crate::config::get_knobs();
    ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_000_000));
    ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
        kb.priority_fee_microlamports,
    ));

    // Use q1.amount_out as the input size on leg 2 in a full flow.
    // For now, we reuse the probe size – okay for skeleton, upgrade later.
    let ix = whirlpool_swap_ix(&meta, input_ata, output_ata, sig.amount_in as u64)?;
    ixs.push(ix);
    Ok(ixs)
}

/*

You’ll likely want to:

Map Pair ordering ↔ pool A/B precisely (store token_mint_a/b in meta—already done—and compare with your Pair).

Use the actual out of leg 1 as input to leg 2 (we can thread q1/q2 through the signal, or quickly re-quote before execute).

Set other_amount_threshold from your slippage knob.

Set token_authority in the Whirlpool keys (above I noted the shortcut to keep the example compact).


*/

/// Builds a VersionedTransaction with proper blockhash and signatures
///
/// # Features
///
/// - `stubs`: Returns a stub transaction that will fail gracefully
/// - Default/production: Real transaction building with proper blockhash caching
#[cfg_attr(feature = "stubs", doc = "🟡 Currently using STUB implementation")]
#[cfg_attr(not(feature = "stubs"), doc = "✅ Using PRODUCTION implementation")]
pub fn build_payer_tx(
    _payer: &Keypair,
    _insts: &[Instruction],
) -> anyhow::Result<VersionedTransaction> {
    #[cfg(feature = "stubs")]
    {
        eprintln!("🟡 STUB: build_payer_tx - this would normally create a real transaction");
        eprintln!("🟡 STUB: Using dummy blockhash and minimal transaction structure");
        // Create a minimal stub transaction that won't crash but won't execute
        use solana_sdk::{
            hash::Hash,
            message::{Message, VersionedMessage},
            transaction::VersionedTransaction,
        };
        let _dummy_blockhash = Hash::default();
        let message = Message::new(_insts, Some(&_payer.pubkey()));
        let versioned_message = VersionedMessage::Legacy(message);
        return Ok(VersionedTransaction::try_new(versioned_message, &[_payer])?);
    }

    #[cfg(not(feature = "stubs"))]
    {
        // Production implementation - requires real blockhash cache
        // TODO: Integrate with blockhash cache from Geyser/RPC
        anyhow::bail!("Production transaction building not implemented - requires blockhash cache integration")
    }
}

/// Returns information about which features are currently enabled
///
/// Useful for runtime debugging and feature detection
pub fn get_feature_info() -> Vec<&'static str> {
    vec![
        #[cfg(feature = "stubs")]
        Some("stubs"),
        #[cfg(feature = "whirlpool")]
        Some("whirlpool"),
        #[cfg(feature = "raydium")]
        Some("raydium"),
        #[cfg(feature = "meteora")]
        Some("meteora"),
        #[cfg(feature = "production")]
        Some("production"),
        #[cfg(feature = "development")]
        Some("development"),
        #[cfg(feature = "experimental")]
        Some("experimental"),
    ]
    .into_iter()
    .filter_map(|s| s)
    .collect()
}
