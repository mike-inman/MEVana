use anyhow::Result;
use solana_sdk::instruction::Instruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

pub struct BundleBuilder {
    max_transactions_per_bundle: usize,
    max_instructions_per_transaction: usize,
}

impl BundleBuilder {
    pub fn new() -> Self {
        Self {
            max_transactions_per_bundle: 5,
            max_instructions_per_transaction: 10,
        }
    }

    pub fn build_arbitrage_bundle(
        &self,
        instructions: Vec<Instruction>,
        payer: &Keypair,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<Vec<Transaction>> {
        let mut transactions = Vec::new();
        let mut current_instructions = Vec::new();

        for instruction in instructions {
            current_instructions.push(instruction);

            // Check if we should create a transaction
            if current_instructions.len() >= self.max_instructions_per_transaction {
                let transaction = Transaction::new_signed_with_payer(
                    &current_instructions,
                    Some(&payer.pubkey()),
                    &[payer],
                    recent_blockhash,
                );
                transactions.push(transaction);
                current_instructions.clear();
            }

            // Check bundle size limit
            if transactions.len() >= self.max_transactions_per_bundle {
                break;
            }
        }

        // Handle remaining instructions
        if !current_instructions.is_empty() && transactions.len() < self.max_transactions_per_bundle {
            let transaction = Transaction::new_signed_with_payer(
                &current_instructions,
                Some(&payer.pubkey()),
                &[payer],
                recent_blockhash,
            );
            transactions.push(transaction);
        }

        log::info!(
            "Built bundle with {} transactions",
            transactions.len()
        );

        Ok(transactions)
    }

    pub fn estimate_bundle_cost(&self, transactions: &[Transaction]) -> u64 {
        // Estimate compute units for the bundle
        transactions.len() as u64 * 200_000 // Rough estimate
    }
}
