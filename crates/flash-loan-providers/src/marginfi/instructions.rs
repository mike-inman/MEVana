use anyhow::Result;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

/// Marginfi Program ID (mainnet)
pub const MARGINFI_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");

/// Marginfi Bank for SOL
pub const MARGINFI_SOL_BANK: Pubkey =
    solana_sdk::pubkey!("CCKtUs6Cgwo4aaQUmBPmyoApH2gUDERxNZCAntD6LYGh");

/// Marginfi Pool for SOL
pub const MARGINFI_SOL_POOL: Pubkey =
    solana_sdk::pubkey!("E8pLkNzPR4QnQdXMLrLbmJNcSQWyDhQsYRjSc9qFCGsS");

const FLASH_LOAN_DISCRIMINATOR: [u8; 8] = [0x9a, 0xe1, 0xc4, 0x7e, 0x8a, 0x0f, 0x2c, 0x1d];
const FLASH_LOAN_REPAY_DISCRIMINATOR: [u8; 8] = [0x2b, 0xf3, 0x4a, 0x6c, 0x9d, 0x8e, 0x1f, 0x7a];

#[repr(C)]
#[derive(Clone, Copy)]
struct FlashLoanInstruction {
    discriminator: [u8; 8],
    amount: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FlashLoanRepayInstruction {
    discriminator: [u8; 8],
    repay_amount: u64,
}

/// Create Marginfi flash loan borrow instruction
pub fn create_flash_loan_instruction(payer: &Pubkey, amount_lamports: u64) -> Result<Instruction> {
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

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(MARGINFI_SOL_BANK, false),
        AccountMeta::new(MARGINFI_SOL_POOL, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Ok(Instruction {
        program_id: MARGINFI_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Create Marginfi flash loan repay instruction
pub fn create_flash_loan_repay_instruction(
    payer: &Pubkey,
    repay_amount: u64,
) -> Result<Instruction> {
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

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(MARGINFI_SOL_BANK, false),
        AccountMeta::new(MARGINFI_SOL_POOL, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: MARGINFI_PROGRAM_ID,
        accounts,
        data,
    })
}

pub fn calculate_flash_loan_fee(loan_amount: u64) -> u64 {
    (loan_amount * 5) / 10_000
}

pub fn validate_flash_loan_preconditions(loan_amount: u64, max_amount: u64) -> Result<()> {
    if loan_amount == 0 {
        anyhow::bail!("Loan amount cannot be zero");
    }

    if loan_amount > max_amount {
        anyhow::bail!("Loan amount {} exceeds maximum {}", loan_amount, max_amount);
    }

    let min_amount = 1_000_000; // 0.001 SOL
    if loan_amount < min_amount {
        anyhow::bail!("Loan amount {} below minimum {}", loan_amount, min_amount);
    }

    Ok(())
}

// Backwards-compatible helper names used by legacy hotpath code
pub fn create_marginfi_flash_loan_instruction(
    payer: &Pubkey,
    amount_lamports: u64,
) -> Result<Instruction> {
    create_flash_loan_instruction(payer, amount_lamports)
}

pub fn create_marginfi_flash_loan_repay_instruction(
    payer: &Pubkey,
    repay_amount: u64,
) -> Result<Instruction> {
    create_flash_loan_repay_instruction(payer, repay_amount)
}

pub fn calculate_marginfi_flash_loan_fee(loan_amount: u64) -> u64 {
    calculate_flash_loan_fee(loan_amount)
}

pub fn validate_marginfi_flash_loan_preconditions(loan_amount: u64, max_amount: u64) -> Result<()> {
    validate_flash_loan_preconditions(loan_amount, max_amount)
}
