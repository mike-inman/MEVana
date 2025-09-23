use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::plugin::{PlanRequest, QuoteRequest};

/// Additional receiver metadata that higher-level services can attach to a plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiverConfig {
    pub receiver_program_id: Pubkey,
    pub receiver_payload: Vec<u8>,
    pub reserve_hint: Option<Pubkey>,
}

impl ReceiverConfig {
    pub fn into_plan_request(self, mut base: PlanRequest) -> PlanRequest {
        base.receiver_program_id = Some(self.receiver_program_id);
        if !self.receiver_payload.is_empty() {
            base.receiver_payload = Some(self.receiver_payload);
        }
        base.reserve_hint = self.reserve_hint;
        base
    }

    pub fn attach_to_quote_request(&self, mut base: QuoteRequest) -> QuoteRequest {
        base.reserve_hint = self.reserve_hint;
        base
    }
}

/// Helper wrapper that bundles the traditional plan request with receiver config.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanRequestWithReceiver {
    pub base: PlanRequest,
    pub receiver: ReceiverConfig,
}

impl PlanRequestWithReceiver {
    pub fn new(base: PlanRequest, receiver: ReceiverConfig) -> Self {
        Self { base, receiver }
    }

    pub fn into_plan_request(self) -> PlanRequest {
        self.receiver.into_plan_request(self.base)
    }
}
