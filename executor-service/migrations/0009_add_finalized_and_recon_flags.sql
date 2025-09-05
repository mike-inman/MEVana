-- Add finalized and reconciliation application columns to execution_results
ALTER TABLE execution_results ADD COLUMN IF NOT EXISTS finalized BOOLEAN DEFAULT false;
ALTER TABLE execution_results ADD COLUMN IF NOT EXISTS finalized_at TIMESTAMPTZ;
ALTER TABLE execution_results ADD COLUMN IF NOT EXISTS reconciled_pnl_applied BOOLEAN DEFAULT false;

-- Backfill: set finalized=true where status='Finalized'
UPDATE execution_results SET finalized=true WHERE status='Finalized' AND finalized=false;
