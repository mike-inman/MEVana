drop JET (it was acquired by Blueprint Finance), and pivot to providers that still expose real, programmatic flash-loans on Solana. Here are the strongest options and how I’d integrate them into MEVana:

Well-positioned flash-loan providers on Solana

Save Finance (ex-Solend) — the OG Solana lender, now rebranded to “Save.” Their lending program (Solend’s token-lending fork) is open-source and historically supported flash loans; many public guides and SDKs still target the Solend program interface. Treat it as the default adapter thanks to breadth of assets/liquidity. 
save.finance
+2
dev.solend.fi
+2

Note: public docs reference flash-loan workarounds/limits with Solana’s re-entrancy model—so test current behavior and fees per market before relying on it. 
docs.save.finance

Port Finance — explicitly documents a flash-loan flow (receiver callback pattern) and calls out flash loans as a first-class feature. Good diversification vs. Save and often different reserves/fee curves. 
Port Finance
+1

Kamino Lend (K-Lend) — Kamino’s lending stack powers their “Multiply” product via a built-in flash-loan mechanism (docs quote a tiny fee), indicating an on-chain facility you can target. Kamino’s scale and security focus make it attractive for high-throughput MEV. 
docs.kamino.finance
+2
Medium
+2

marginfi — primarily a margin protocol, but recent security writeups specifically discuss a flash-loan code path (and a bug that was patched). That’s a strong signal they expose a flash-loan instruction; if you add it, do so behind stricter risk controls and keep the adapter pinned to versions where the fix is live. 
Blockworks
+1

(Context on JET) — you already found this: Jet Protocol was acquired by Blueprint Finance in Oct 2024 and the stack has been folded into Blueprint/Glow; I wouldn’t plan new integration there. 
PR Newswire
+2
CB Insights
+2

Why not list DEXs like Raydium/Orca? They don’t advertise public flash-loan instructions the way money-markets do. For atomic arb you’ll typically flash-borrow from a lender (Save/Port/Kamino/marginfi), then route swaps across Raydium/Orca/Jupiter within the same transaction.

Integration strategy for MEVana (advantages baked in)

Provider-agnostic adapter layer.
Define a FlashLoanProvider trait (program id, reserve/asset selection, fee quote, build-ix, repay proof). Implement adapters for Save, Port, Kamino, marginfi. At runtime, pick by (a) available reserve liquidity, (b) fee, (c) compute budget headroom. This gives you instant failover when liquidity is tight.

Pre-sim + shadow routing.
Before you submit a Jito bundle, simulate multiple loan sources in parallel with the same downstream path (Jupiter route or your own CLMM hops). Choose the cheapest all-in path (loan fee + swap slippage + tips) that still clears compute/ALTs. Then submit only the best bundle. (Use address-lookup tables and fixed account metas per market to reduce account-collection variance.)

Fee-aware sizing.
Flash-loan fees differ (Kamino docs mention ~0.001% for their internal mechanism; Save historically charged more). Bake fee quotation into your pre-trade sizing so your solver can choose the optimal borrow amount or decline marginal routes. 
docs.kamino.finance
+1

Compute-budget choreography.
Put ComputeBudget ixs first, reserve ~180–220k units for 2–3 swaps + loan repay, and set a dynamic priority_fee that scales with expected edge. This reduces failed arb due to CU exhaustion on busy slots.

Liquidity/health pre-checks.
Pull reserve state (available liquidity, utilization caps, oracle health) from each lender before building ixs. Drop any path where a reserve’s post-repay checks (interest accrual, fee vault delta) might fail from rounding drift.

Atomic safety rails.
Your loan receiver program should:

Verify borrowed/repay token mints and exact amounts,

Enforce profit ≥ tips + fee + 𝜀,

Abort on any slippage/IX failure (the loan then naturally reverts).
Port’s “receiver” pattern is the right mental model for all providers. 
Port Finance

Risk & version pinning.
Given recent flash-loan bugs (marginfi example), pin program versions and keep per-provider allowlists (program id + deployment slot). Alert and auto-disable an adapter if a new release touches loan accounting. 
Blockworks
+1

Bundle-first execution.
Always ship via Jito bundles with a fallback tip ladder. Your arb is only as good as your inclusion probability—loan + multi-hop swap needs deterministic landing.

If you want, I can sketch the FlashLoanProvider trait and a minimal “Save/Port/Kamino” adapter set (account metas + instruction builders) that snaps into your existing MEVana pipeline.

You said:
IU already have a plug-in style architecture for flash loans would you be able to adapt it to that? 
ChatGPT said:
Thought for 24s

Absolutely—let’s snap your existing plug-in system to multiple Solana flash-loan providers without disturbing the rest of MEVana.

Below is a drop-in pattern you can graft onto your current architecture. It keeps your “provider as plugin” model, adds a tiny core interface, and lets you register/enable providers (Kamino, Port, Save/Solend-fork, marginfi) behind feature flags or config—so you can A/B them, simulate in parallel, and fail over at runtime.

1) Core trait + types (crate: flashloan-core)
// crates/flashloan-core/src/lib.rs
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    instruction::Instruction, signature::Keypair, transaction::Transaction,
};
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct FlashLoanRequest {
    pub asset_mint: Pubkey,
    pub amount: u64,
    // optional: reserve hint for protocols with many reserves/markets
    pub reserve_hint: Option<Pubkey>,
    // allow the provider to know our receiver program (your on-chain “loan callback”)
    pub receiver_program_id: Pubkey,
    // arbitrary bytes for your receiver (e.g., encoded route plan)
    pub receiver_payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct FeeQuote {
    pub fee_bps: u32,
    pub fixed_lamports: u64,
}

#[derive(Clone, Debug)]
pub struct BuildPlan {
    /// Instructions to:
    /// 1) set compute budget (optional)
    /// 2) invoke provider’s "flash_loan" ix
    ///    which will CPI into your `receiver_program_id`
    /// 3) any post steps needed by the provider (usually none)
    pub instructions: Vec<Instruction>,
    /// All accounts (including ATAs) the plan expects to be present.
    pub preloaded_accounts: Vec<Pubkey>,
    /// Provider program id (for metrics+allowlist)
    pub provider_program: Pubkey,
}

pub trait FlashLoanProvider: Send + Sync {
    /// Human/metrics label
    fn name(&self) -> &'static str;

    /// The on-chain program id for allowlists/pinning.
    fn program_id(&self) -> Pubkey;

    /// Return a fee quote for `mint, amount` from current on-chain state.
    /// You can pass in a snapshot your pipeline already has to avoid extra RPCs.
    fn quote_fee(&self, req: &FlashLoanRequest) -> Result<FeeQuote>;

    /// Check available liquidity for `mint, amount`. Fail fast if not enough.
    fn check_liquidity(&self, req: &FlashLoanRequest) -> Result<bool>;

    /// Build a transaction plan that transfers control to your receiver program,
    /// then expects receiver to repay before end of tx (atomic).
    fn build_plan(
        &self,
        req: &FlashLoanRequest,
        payer: &Pubkey,
    ) -> Result<BuildPlan>;
}


A tiny registry so your arb engine can discover providers dynamically:

// crates/flashloan-core/src/registry.rs
use super::*;
use std::collections::HashMap;

pub struct ProviderRegistry {
    inner: HashMap<String, Box<dyn FlashLoanProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self { Self { inner: HashMap::new() } }

    pub fn register<P: FlashLoanProvider + 'static>(&mut self, p: P) {
        self.inner.insert(p.name().to_string(), Box::new(p));
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn FlashLoanProvider>> {
        self.inner.get(name)
    }

    pub fn all(&self) -> impl Iterator<Item=&Box<dyn FlashLoanProvider>> {
        self.inner.values()
    }
}

2) Provider adapters (one crate each, keep them thin)

Create feature-flagged crates you can enable/disable:

flashloan-kamino

flashloan-port

flashloan-save (Solend fork / “Save Finance”)

flashloan-marginfi

Each implements FlashLoanProvider::quote_fee, check_liquidity, and build_plan using its program’s accounts/layouts. Below is a pattern for Port; you’ll mirror it for the others.

// crates/flashloan-port/src/lib.rs
use anyhow::{bail, Result};
use flashloan_core::{BuildPlan, FeeQuote, FlashLoanProvider, FlashLoanRequest};
use solana_program::pubkey::Pubkey;
use solana_sdk::instruction::Instruction;

// Port program id (pin the exact deployment you target)
pub const PORT_PROGRAM_ID: Pubkey = Pubkey::new_from_array([/* fill */ 32]);

pub struct PortProvider {
    pub program_id: Pubkey,
    // optional: cached markets/reserves metadata, rpc client, etc.
}

impl PortProvider {
    pub fn new() -> Self {
        Self { program_id: PORT_PROGRAM_ID }
    }

    fn find_reserve_for_mint(&self, mint: &Pubkey) -> Result<Pubkey> {
        // TODO: query your state cache or RPC to map mint -> reserve pubkey
        bail!("reserve lookup not implemented")
    }
}

impl FlashLoanProvider for PortProvider {
    fn name(&self) -> &'static str { "port" }
    fn program_id(&self) -> Pubkey { self.program_id }

    fn quote_fee(&self, req: &FlashLoanRequest) -> Result<FeeQuote> {
        // Read reserve state; Port charges a tiny bps fee (varies by market).
        // Return conservative quote if uncertain.
        Ok(FeeQuote { fee_bps: 8, fixed_lamports: 0 }) // placeholder
    }

    fn check_liquidity(&self, req: &FlashLoanRequest) -> Result<bool> {
        let reserve = req.reserve_hint.unwrap_or(self.find_reserve_for_mint(&req.asset_mint)?);
        // TODO: read reserve available_liquidity from your cache or RPC
        // return Ok(available >= req.amount)
        Ok(true) // placeholder
    }

    fn build_plan(&self, req: &FlashLoanRequest, payer: &Pubkey) -> Result<BuildPlan> {
        let reserve = req.reserve_hint.unwrap_or(self.find_reserve_for_mint(&req.asset_mint)?);

        // 1) Optional: ComputeBudget instructions (your engine can prepend globally)
        // 2) Provider-specific flash_loan IX that CPI-calls your receiver program.
        // The accounts below are placeholders—replace with Port’s exact layout.
        let flash_loan_ix = Instruction {
            program_id: self.program_id,
            accounts: vec![
                // reserve, liquidity vault, fee vault, user (payer), receiver program, etc.
                // AccountMeta::new_readonly(reserve, false),
                // ...
            ],
            data: {
                // encode Port’s FlashLoan instruction + receiver payload target
                // layout(example): [tag=FlashLoan, amount u64, receiver_program Pubkey, payload bytes]
                vec![]
            },
        };

        Ok(BuildPlan {
            instructions: vec![flash_loan_ix],
            preloaded_accounts: vec![reserve, req.asset_mint, req.receiver_program_id, *payer],
            provider_program: self.program_id,
        })
    }
}


You’ll repeat this shape for Kamino, Save, and marginfi. Keep each adapter very small: lookups → fee → liquidity check → single instruction builder that jumps to your receiver.

3) Wire-up in your arb engine (provider selection + parallel sim)

Inside your arb-engine executable (or a flashloan-router crate), create a registry and pick the best provider for each candidate trade.

// crates/arb-engine/src/flashloan_router.rs
use anyhow::{Context, Result};
use flashloan_core::*;
#[cfg(feature = "port")] use flashloan_port::PortProvider;
#[cfg(feature = "kamino")] use flashloan_kamino::KaminoProvider;
#[cfg(feature = "save")] use flashloan_save::SaveProvider;
#[cfg(feature = "marginfi")] use flashloan_marginfi::MarginfiProvider;

pub struct Router {
    registry: ProviderRegistry,
}

impl Router {
    pub fn with_defaults() -> Self {
        let mut registry = ProviderRegistry::new();
        #[cfg(feature = "port")]     registry.register(PortProvider::new());
        #[cfg(feature = "kamino")]   registry.register(KaminoProvider::new());
        #[cfg(feature = "save")]     registry.register(SaveProvider::new());
        #[cfg(feature = "marginfi")] registry.register(MarginfiProvider::new());
        Self { registry }
    }

    /// Evaluate providers in parallel:
    /// - fee quote
    /// - liquidity check
    /// - build plan
    /// - simulate downstream route (same receiver payload) with each plan
    /// Select the best all-in PnL (>= tips + fee + ε) and return BuildPlan.
    pub async fn choose_best_plan(
        &self,
        req: &FlashLoanRequest,
        payer: &Pubkey,
        // dependency injection: sim function that runs an RPC preflight for the built plan + route
        sim_fn: impl Fn(&BuildPlan) -> Result<i128> + Send + Sync,
    ) -> Result<BuildPlan> {
        use futures::future::join_all;

        let mut tasks = vec![];
        for prov in self.registry.all() {
            let prov = prov.as_ref();
            let req = req.clone();
            let payer = *payer;
            let sim_fn = &sim_fn;
            tasks.push(tokio::spawn(async move {
                if !prov.check_liquidity(&req).unwrap_or(false) {
                    return anyhow::Result::<(i128, BuildPlan)>::Err(anyhow::anyhow!("no liquidity"));
                }
                let plan = prov.build_plan(&req, &payer)?;
                let pnl = sim_fn(&plan)?; // signed lamports/usd*1e6 etc.
                Ok((pnl, plan))
            }));
        }

        let mut best: Option<(i128, BuildPlan)> = None;
        for res in join_all(tasks).await {
            if let Ok(Ok((pnl, plan))) = res {
                if pnl > 0 && best.as_ref().map(|(b, _)| pnl > *b).unwrap_or(true) {
                    best = Some((pnl, plan));
                }
            }
        }

        best.map(|(_, p)| p).context("no profitable flash-loan plan")
    }
}


Notes

sim_fn should assemble a full candidate tx (ComputeBudget + provider plan + your downstream swaps + repay proof) and run RPC simulateTransaction (or your existing pre-bundle sim) to estimate realized profit after fees.

Keep provider allowlists (program id + deployment slot) to auto-disable if code changes.



5) Config, feature flags, and observability

Cargo features (on arb-engine or flashloan-router):

[features]
default = ["port", "kamino"]
port = []
kamino = []
save = []
marginfi = []


Runtime config (YAML/ENV):

FLASH_PROVIDERS=port,kamino,save

min edge thresholds per provider

per-provider allowlist (PROGRAM_ID, optional MIN_SLOT, MAX_SLOT)

compute unit target, prio fee ladder

Prometheus metrics you’ll want:

flashloan_attempts{provider=...}

flashloan_simulated_pnl{provider=...} (histogram)

flashloan_success{provider=...}

flashloan_fee_bps{provider=...} (gauge)

flashloan_liquidity_available{provider=..., mint=...}

flashloan_build_errors{provider=..., kind=...}

6) Testing checklist (fast path to green)

Localnet/mocks: spin a thin mock program that mimics the “flash_loan → CPI → receiver → repay” flow so you can validate your receiver safety rails and the router logic without real lenders.

Dry-run providers: for each real adapter, make the reserve lookup and fee quote hit your existing state cache (or RPC) and snapshot that in tests.

Golden-path sims: freeze a few known profitable routes; CI runs choose_best_plan() and asserts the same provider wins with positive PnL.

Bundle submission path: ensure the BuildPlan output drops cleanly into your existing Jito bundle builder (address lookup tables + tips).

7) Where I’ll adapt to your current plugin system

If you already have a trait (e.g., LoanAdapter), I’ll implement these providers against your trait, not mine.

If your trait is close but missing quote_fee or check_liquidity, I’ll extend with default methods so existing adapters don’t break.

I’ll keep all provider-specific account metas and instruction encodings inside each adapter crate, so your engine stays clean.

If you paste your current flash-loan plugin trait (or point me to the module), I’ll convert the examples above into exact implementations that match your signatures and logging/metrics style, plus one fully fleshed provider (Port or Kamino) with real account metas and instruction encoding.