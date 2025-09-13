use anyhow::Result;
use log::info;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar,
};

/// Marginfi Program ID (mainnet)
pub const MARGINFI_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");

/// Marginfi Bank for SOL
pub const MARGINFI_SOL_BANK: Pubkey = solana_sdk::pubkey!("CCKtUs6Cgwo4aaQUmBPmyoApH2gUDERxNZCAntD6LYGh");

/// Marginfi Pool for SOL 
pub const MARGINFI_SOL_POOL: Pubkey = solana_sdk::pubkey!("E8pLkNzPR4QnQdXMLrLbmJNcSQWyDhQsYRjSc9qFCGsS");

/// Flash Loan Instruction Data Discriminator (8 bytes)
const FLASH_LOAN_DISCRIMINATOR: [u8; 8] = [0x9a, 0xe1, 0xc4, 0x7e, 0x8a, 0x0f, 0x2c, 0x1d];
const FLASH_LOAN_REPAY_DISCRIMINATOR: [u8; 8] = [0x2b, 0xf3, 0x4a, 0x6c, 0x9d, 0x8e, 0x1f, 0x7a];

/// Marginfi Flash Loan Account Structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlashLoanInstruction {
    pub discriminator: [u8; 8],
    pub amount: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlashLoanRepayInstruction {
    pub discriminator: [u8; 8],
    pub repay_amount: u64,
}

/// Create Marginfi flash loan borrow instruction
pub fn create_marginfi_flash_loan_instruction(
    payer: &Pubkey,
    amount_lamports: u64,
) -> Result<Instruction> {
    info!("Creating Marginfi flash loan instruction for {} lamports", amount_lamports);

    // Flash loan instruction data
    let instruction_data = FlashLoanInstruction {
        discriminator: FLASH_LOAN_DISCRIMINATOR,
        amount: amount_lamports,
    };

    let data = unsafe {
        std::slice::from_raw_parts(
            &instruction_data as *const _ as *const u8,
            std::mem::size_of::<FlashLoanInstruction>(),
        )
        .to_vec()
    };

    // Marginfi flash loan accounts
    let accounts = vec![
        // Payer account (signer)
        AccountMeta::new(*payer, true),
        // Marginfi program bank account
        AccountMeta::new(MARGINFI_SOL_BANK, false),
        // Marginfi pool account  
        AccountMeta::new(MARGINFI_SOL_POOL, false),
        // System program
        AccountMeta::new_readonly(system_program::id(), false),
        // Rent sysvar
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        // Clock sysvar
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Ok(Instruction {
        program_id: MARGINFI_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Create Marginfi flash loan repay instruction
pub fn create_marginfi_flash_loan_repay_instruction(
    payer: &Pubkey,
    repay_amount: u64,
) -> Result<Instruction> {
    info!("Creating Marginfi flash loan repay instruction for {} lamports", repay_amount);

    // Repay instruction data
    let instruction_data = FlashLoanRepayInstruction {
        discriminator: FLASH_LOAN_REPAY_DISCRIMINATOR,
        repay_amount,
    };

    let data = unsafe {
        std::slice::from_raw_parts(
            &instruction_data as *const _ as *const u8,
            std::mem::size_of::<FlashLoanRepayInstruction>(),
        )
        .to_vec()
    };

    // Marginfi repay accounts (similar to borrow but different instruction)
    let accounts = vec![
        // Payer account (signer)
        AccountMeta::new(*payer, true),
        // Marginfi program bank account
        AccountMeta::new(MARGINFI_SOL_BANK, false),
        // Marginfi pool account
        AccountMeta::new(MARGINFI_SOL_POOL, false),
        // System program
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: MARGINFI_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Create user token account for flash loan operations
pub fn create_user_token_account_instruction(
    _payer: &Pubkey,
    token_mint: &Pubkey,
) -> Result<Instruction> {
    // This would create the associated token account for the user
    // For SOL, this is typically not needed as SOL is held directly in the wallet
    // But for other tokens, we'd need to create the ATA
    
    info!("Creating user token account for mint: {}", token_mint);
    
    // For now, return a placeholder instruction
    // In production, use spl_associated_token_account::instruction::create_associated_token_account
    Ok(Instruction {
        program_id: system_program::id(),
        accounts: vec![],
        data: vec![],
    })
}

/// Calculate Marginfi flash loan fee (5 basis points = 0.05%)
pub fn calculate_marginfi_flash_loan_fee(loan_amount: u64) -> u64 {
    // Marginfi charges 0.05% (5 bps) flash loan fee
    (loan_amount * 5) / 10_000
}

/// Validate Marginfi flash loan preconditions
pub fn validate_marginfi_flash_loan_preconditions(
    loan_amount: u64,
    max_amount: u64,
) -> Result<()> {
    if loan_amount == 0 {
        return Err(anyhow::anyhow!("Loan amount cannot be zero"));
    }
    
    if loan_amount > max_amount {
        return Err(anyhow::anyhow!(
            "Loan amount {} exceeds maximum {}", 
            loan_amount, 
            max_amount
        ));
    }
    
    // Minimum reasonable flash loan amount (to cover fees)
    let min_amount = 1_000_000; // 0.001 SOL
    if loan_amount < min_amount {
        return Err(anyhow::anyhow!(
            "Loan amount {} below minimum {}", 
            loan_amount, 
            min_amount
        ));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_marginfi_fee_calculation() {
        // 1 SOL loan should cost 0.0005 SOL (500,000 lamports)
        let loan_amount = 1_000_000_000; // 1 SOL
        let expected_fee = 500_000; // 0.0005 SOL
        
        assert_eq!(calculate_marginfi_flash_loan_fee(loan_amount), expected_fee);
    }
    
    #[test]
    fn test_validation() {
        let max_amount = 10_000_000_000; // 10 SOL
        
        // Valid loan
        assert!(validate_marginfi_flash_loan_preconditions(5_000_000_000, max_amount).is_ok());
        
        // Zero loan
        assert!(validate_marginfi_flash_loan_preconditions(0, max_amount).is_err());
        
        // Exceeds maximum
        assert!(validate_marginfi_flash_loan_preconditions(15_000_000_000, max_amount).is_err());
        
        // Below minimum
        assert!(validate_marginfi_flash_loan_preconditions(500_000, max_amount).is_err());
    }
}