#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use core::str;
use heapless::String;
use solana_program::{
    account_info::AccountInfo,
    declare_id, entrypoint,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use thiserror::Error;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

declare_id!("26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg");

/// Payload forwarded to the on-chain flash-loan receiver program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteFlashLoanPayload {
    /// Provider identifier (e.g. "portfinance", "jetprotocol").
    pub provider_id: String<32>,
    /// Requested amount in the provider's base token (lamports for SOL).
    pub amount: u64,
    /// Token mint for the loan (WSOL for current providers).
    pub token_mint: Pubkey,
    /// Arbitrary payload forwarded to the downstream receiver program.
    pub payload: Vec<u8>,
}

impl ExecuteFlashLoanPayload {
    pub fn pack(&self) -> Vec<u8> {
        let mut data =
            Vec::with_capacity(1 + self.provider_id.len() + 8 + 32 + 4 + self.payload.len());
        data.push(self.provider_id.len() as u8);
        data.extend_from_slice(self.provider_id.as_bytes());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(self.token_mint.as_ref());
        data.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&self.payload);
        data
    }

    pub fn unpack(data: &[u8]) -> Result<Self, FlashLoanReceiverError> {
        if data.is_empty() {
            return Err(FlashLoanReceiverError::Serialization);
        }
        let len = data[0] as usize;
        if data.len() < 1 + len + 8 + 32 + 4 {
            return Err(FlashLoanReceiverError::Serialization);
        }
        let mut provider = String::<32>::new();
        let name_slice = &data[1..1 + len];
        let name = str::from_utf8(name_slice).map_err(|_| FlashLoanReceiverError::Serialization)?;
        provider
            .push_str(name)
            .map_err(|_| FlashLoanReceiverError::ProviderIdTooLong)?;

        let amount_offset = 1 + len;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&data[amount_offset..amount_offset + 8]);
        let amount = u64::from_le_bytes(amount_bytes);
        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&data[amount_offset + 8..amount_offset + 8 + 32]);
        let token_mint = Pubkey::new_from_array(mint_bytes);
        let payload_offset = amount_offset + 8 + 32;
        let mut payload_len_bytes = [0u8; 4];
        payload_len_bytes.copy_from_slice(&data[payload_offset..payload_offset + 4]);
        let payload_len = u32::from_le_bytes(payload_len_bytes) as usize;
        if data.len() < payload_offset + 4 + payload_len {
            return Err(FlashLoanReceiverError::Serialization);
        }
        let payload = data[payload_offset + 4..payload_offset + 4 + payload_len].to_vec();
        Ok(Self {
            provider_id: provider,
            amount,
            token_mint,
            payload,
        })
    }
}

#[derive(Debug, Error)]
pub enum FlashLoanReceiverError {
    #[error("provider id exceeds 32 characters")]
    ProviderIdTooLong,
    #[error("serialization failure")]
    Serialization,
}

impl From<FlashLoanReceiverError> for ProgramError {
    fn from(value: FlashLoanReceiverError) -> Self {
        ProgramError::Custom(value as u32)
    }
}

/// Build an instruction that asks the flash-loan receiver program to co-ordinate
/// a loan for the downstream provider.
pub fn build_execute_flash_loan_ix(
    program_id: &Pubkey,
    executor_authority: Pubkey,
    provider_id: &str,
    token_mint: Pubkey,
    amount: u64,
    mut extra_accounts: Vec<AccountMeta>,
    receiver_payload: Vec<u8>,
) -> Result<Instruction, FlashLoanReceiverError> {
    if provider_id.len() > 32 {
        return Err(FlashLoanReceiverError::ProviderIdTooLong);
    }

    let mut provider = String::<32>::new();
    provider
        .push_str(provider_id)
        .map_err(|_| FlashLoanReceiverError::ProviderIdTooLong)?;

    let payload = ExecuteFlashLoanPayload {
        provider_id: provider,
        amount,
        token_mint,
        payload: receiver_payload,
    };

    let mut accounts = Vec::with_capacity(1 + extra_accounts.len());
    accounts.push(AccountMeta::new(executor_authority, true));
    accounts.append(&mut extra_accounts);

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: payload.pack(),
    })
}

/// Placeholder processor. The actual on-chain program will decode the payload
/// and perform the CPI dance (borrow -> trade -> repay).
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let payload = ExecuteFlashLoanPayload::unpack(instruction_data)?;
    msg!(
        "flash-loan-receiver stub invoked: provider={}, amount={}, token={}",
        payload.provider_id,
        payload.amount,
        payload.token_mint
    );
    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;
    use solana_program::pubkey::Pubkey;

    #[test]
    fn payload_round_trip() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ix = build_execute_flash_loan_ix(
            &program_id,
            authority,
            "jetprotocol",
            mint,
            42,
            Vec::new(),
            vec![1, 2, 3],
        )
        .expect("instruction built");
        assert_eq!(ix.program_id, program_id);
        assert_eq!(ix.accounts[0].pubkey, authority);
        let decoded = ExecuteFlashLoanPayload::unpack(&ix.data).unwrap();
        assert_eq!(decoded.provider_id.as_str(), "jetprotocol");
        assert_eq!(decoded.amount, 42);
        assert_eq!(decoded.token_mint, mint);
        assert_eq!(decoded.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_id() {
        // Ensure the declared id is a valid base58 pubkey
        let expected = Pubkey::from_str("26hsTvfGVS5yxfgYM7htEgrsCKdvMVtWSjHnyk277hCg").unwrap();
        assert_eq!(crate::id(), expected);
    }
}
