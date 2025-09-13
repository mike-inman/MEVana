-- Add token delta valuation columns for reconciliation
ALTER TABLE execution_reconciliation
    ADD COLUMN IF NOT EXISTS token_delta_lamports BIGINT;

ALTER TABLE execution_reconciliation
    ADD COLUMN IF NOT EXISTS total_reconciled_lamports BIGINT; -- sol_delta_lamports + token_delta_lamports (best-effort)

-- Backfill existing rows where possible (assume existing reconciled was SOL-only)
UPDATE execution_reconciliation
SET total_reconciled_lamports = sol_delta_lamports
WHERE total_reconciled_lamports IS NULL;

-- No indexes yet; add later if querying by these columns proves necessary.
