use anyhow::{anyhow, Result};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

/// Static account wiring for a Solend reserve that supports flash loans.
#[derive(Debug, Clone)]
pub struct SolendReserveConfig {
    pub program_id: Pubkey,
    pub lending_market: Pubkey,
    pub reserve: Pubkey,
    pub reserve_liquidity_supply: Pubkey,
    pub lending_market_authority: Pubkey,
    pub fee_receiver: Pubkey,
    pub host_fee_receiver: Pubkey,
    pub token_program_id: Pubkey,
}

impl SolendReserveConfig {
    /// Ensure the provided market authority matches the PDA Solend expects.
    pub fn validate(&self) -> Result<()> {
        let (derived_authority, _) =
            Pubkey::find_program_address(&[self.lending_market.as_ref()], &self.program_id);
        if derived_authority != self.lending_market_authority {
            return Err(anyhow!(
                "solend configuration mismatch: derived authority {} != provided {}",
                derived_authority,
                self.lending_market_authority
            ));
        }
        Ok(())
    }

    pub fn host_fee_receiver(&self) -> Pubkey {
        if self.host_fee_receiver == Pubkey::default() {
            self.fee_receiver
        } else {
            self.host_fee_receiver
        }
    }
}

const FLASH_BORROW_DISCRIMINATOR: u8 = 19;
const FLASH_REPAY_DISCRIMINATOR: u8 = 20;

pub fn create_flash_borrow_instruction(
    config: &SolendReserveConfig,
    borrower_liquidity_account: Pubkey,
    liquidity_amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 8);
    data.push(FLASH_BORROW_DISCRIMINATOR);
    data.extend_from_slice(&liquidity_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(config.reserve_liquidity_supply, false),
        AccountMeta::new(borrower_liquidity_account, false),
        AccountMeta::new(config.reserve, false),
        AccountMeta::new_readonly(config.lending_market, false),
        AccountMeta::new_readonly(config.lending_market_authority, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(config.token_program_id, false),
    ];

    Instruction {
        program_id: config.program_id,
        accounts,
        data,
    }
}

pub fn create_flash_repay_instruction(
    config: &SolendReserveConfig,
    borrower_liquidity_account: Pubkey,
    borrower_authority: Pubkey,
    liquidity_amount: u64,
    borrow_instruction_index: u8,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 8 + 1);
    data.push(FLASH_REPAY_DISCRIMINATOR);
    data.extend_from_slice(&liquidity_amount.to_le_bytes());
    data.push(borrow_instruction_index);

    let accounts = vec![
        AccountMeta::new(borrower_liquidity_account, false),
        AccountMeta::new(config.reserve_liquidity_supply, false),
        AccountMeta::new(config.fee_receiver, false),
        AccountMeta::new(config.host_fee_receiver(), false),
        AccountMeta::new(config.reserve, false),
        AccountMeta::new_readonly(config.lending_market, false),
        AccountMeta::new_readonly(borrower_authority, true),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(config.token_program_id, false),
    ];

    Instruction {
        program_id: config.program_id,
        accounts,
        data,
    }
}
