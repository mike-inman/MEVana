-- Add reconciled profit column to execution_results
ALTER TABLE execution_results
    ADD COLUMN IF NOT EXISTS reconciled_profit_lamports BIGINT;

-- Backfill existing rows with actual_profit_lamports where present (one-time noop safe)
UPDATE execution_results
SET reconciled_profit_lamports = actual_profit_lamports
WHERE reconciled_profit_lamports IS NULL;