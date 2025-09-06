use crate::models::SwapEvent;
// use normalizer_runtime::NormalizedSwapV1; // this is your generated/parsed struct
use crate::NormalizedSwapV1;

pub fn to_swap_event(ns: &NormalizedSwapV1) -> SwapEvent {
    SwapEvent {
        dex: ns.dex.clone(),
        pool_id: ns.pool_id.clone(),
        slot: ns.slot,
        sig: ns.signature.clone(),
        in_mint: ns.token_in_mint.clone(),
        out_mint: ns.token_out_mint.clone(),
        amount_in: ns.amount_in.to_string(),
        amount_out: ns.amount_out.to_string(),
        fee_paid: ns
            .fee_bps_estimate
            .map(|b| b as u64)
            .unwrap_or_default()
            .to_string(),
    }
}
