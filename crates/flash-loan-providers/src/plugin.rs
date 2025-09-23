use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::borrow::Cow;
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProviderId(Cow<'static, str>);

impl ProviderId {
    pub const fn new(id: &'static str) -> Self {
        Self(Cow::Borrowed(id))
    }

    pub fn owned(id: String) -> Self {
        Self(Cow::Owned(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_cow(&self) -> Cow<'static, str> {
        self.0.clone()
    }
}

impl From<String> for ProviderId {
    fn from(value: String) -> Self {
        ProviderId::owned(value)
    }
}

impl From<&str> for ProviderId {
    fn from(value: &str) -> Self {
        ProviderId::owned(value.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum Cluster {
    #[default]
    Mainnet,
    Devnet,
    Custom(&'static str),
}

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub cluster: Cluster,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub token_mint: Pubkey,
    pub amount: u64,
    /// Optional reserve hint when a protocol exposes multiple pools per mint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_hint: Option<Pubkey>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub provider: ProviderId,
    pub token_mint: Pubkey,
    pub available_amount: u64,
    pub fee_bps: u16,
    pub fee_lamports: u64,
    pub estimated_latency_ms: u16,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanRequest {
    pub token_mint: Pubkey,
    pub amount: u64,
    pub borrower: Pubkey,
    /// Downstream receiver program invoked during the flash-loan callback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_program_id: Option<Pubkey>,
    /// Arbitrary payload forwarded to the receiver (route plan, safety config, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_payload: Option<Vec<u8>>,
    /// Provider-specific reserve hint to speed up account discovery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_hint: Option<Pubkey>,
}

#[derive(Clone, Debug)]
pub struct PlanInstructions {
    pub setup: Vec<Instruction>,
    pub borrow: Vec<Instruction>,
    pub repay: Vec<Instruction>,
    pub cleanup: Vec<Instruction>,
}

impl PlanInstructions {
    pub fn new() -> Self {
        Self {
            setup: Vec::new(),
            borrow: Vec::new(),
            repay: Vec::new(),
            cleanup: Vec::new(),
        }
    }
}

impl Default for PlanInstructions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct PlanResponse {
    pub provider: ProviderId,
    pub amount: u64,
    pub fee_lamports: u64,
    pub instructions: PlanInstructions,
}

#[async_trait]
pub trait FlashLoanProvider: Send + Sync + Debug {
    fn id(&self) -> ProviderId;
    fn program_id(&self) -> Pubkey;
    fn display_name(&self) -> Cow<'static, str> {
        self.id().as_cow()
    }

    fn supports_token(&self, _mint: &Pubkey) -> bool {
        true
    }

    fn max_loan_amount(&self) -> Option<u64> {
        None
    }

    async fn quote(
        &self,
        environment: &Environment,
        request: &QuoteRequest,
    ) -> Result<Option<QuoteResponse>>;

    async fn build_plan(
        &self,
        environment: &Environment,
        request: &PlanRequest,
    ) -> Result<PlanResponse>;
}
