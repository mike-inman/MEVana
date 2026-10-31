-- 2026-10-31: Local research evidence schema. Safe to apply repeatedly.
CREATE SCHEMA IF NOT EXISTS research;
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'research_reader') THEN
    CREATE ROLE research_reader NOLOGIN;
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS research.experiments (
  result_id text PRIMARY KEY,
  dataset_id text NOT NULL,
  dataset_metadata_version text,
  request_contract_version integer,
  evidence_kind text NOT NULL,
  status text NOT NULL,
  window_start_utc timestamptz NOT NULL,
  window_end_utc timestamptz NOT NULL,
  strategy_version text NOT NULL,
  engine_version text NOT NULL,
  cost_model_version text NOT NULL,
  request_canonicalization_version integer NOT NULL,
  request_sha256 text NOT NULL,
  quote_mint text NOT NULL,
  quote_symbol text NOT NULL,
  quote_decimals integer NOT NULL CHECK (quote_decimals BETWEEN 0 AND 18),
  scope jsonb NOT NULL,
  scope_label text NOT NULL,
  source_closeout text NOT NULL,
  limitations jsonb NOT NULL,
  seed_loaded_at timestamptz NOT NULL
);
ALTER TABLE research.experiments ADD COLUMN IF NOT EXISTS dataset_metadata_version text;
ALTER TABLE research.experiments ADD COLUMN IF NOT EXISTS request_contract_version integer;
CREATE TABLE IF NOT EXISTS research.policy_results (
  result_id text NOT NULL REFERENCES research.experiments(result_id) ON DELETE CASCADE,
  policy_kind text NOT NULL,
  candidates_evaluated bigint NOT NULL,
  opportunities_detected bigint NOT NULL,
  selected_or_executed bigint NOT NULL,
  missed_candidates bigint,
  estimated_net_profit_quote_atoms bigint,
  simulated_net_profit_quote_atoms bigint,
  realized_net_profit_quote_atoms bigint,
  unevaluable_candidates bigint NOT NULL,
  evaluable_candidates bigint NOT NULL,
  PRIMARY KEY (result_id, policy_kind)
);
CREATE TABLE IF NOT EXISTS research.policy_rejections (
  result_id text NOT NULL,
  policy_kind text NOT NULL,
  reason_code text NOT NULL,
  rejected_count bigint NOT NULL,
  is_unevaluable boolean NOT NULL CHECK (is_unevaluable = (reason_code IN ('state_gap','insufficient_state','tick_array_absent'))),
  PRIMARY KEY (result_id, policy_kind, reason_code),
  FOREIGN KEY (result_id, policy_kind) REFERENCES research.policy_results(result_id, policy_kind) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS research.shape_results (
  result_id text NOT NULL REFERENCES research.experiments(result_id) ON DELETE CASCADE,
  shape text NOT NULL,
  policy_kind text NOT NULL,
  route_count integer NOT NULL,
  candidates_evaluated bigint NOT NULL,
  opportunities_detected bigint NOT NULL,
  selected_or_executed bigint NOT NULL,
  missed_candidates bigint,
  estimated_net_profit_quote_atoms bigint,
  simulated_net_profit_quote_atoms bigint,
  realized_net_profit_quote_atoms bigint,
  unevaluable_candidates bigint NOT NULL,
  evaluable_candidates bigint NOT NULL,
  simulation_unavailable jsonb NOT NULL,
  PRIMARY KEY (result_id, shape, policy_kind)
);
CREATE TABLE IF NOT EXISTS research.shape_rejections (
  result_id text NOT NULL,
  shape text NOT NULL,
  policy_kind text NOT NULL,
  reason_code text NOT NULL,
  rejected_count bigint NOT NULL,
  is_unevaluable boolean NOT NULL CHECK (is_unevaluable = (reason_code IN ('state_gap','insufficient_state','tick_array_absent'))),
  PRIMARY KEY (result_id, shape, policy_kind, reason_code),
  FOREIGN KEY (result_id, shape, policy_kind) REFERENCES research.shape_results(result_id, shape, policy_kind) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS research.pool_direction_probes (
  result_id text NOT NULL REFERENCES research.experiments(result_id) ON DELETE CASCADE,
  pool_id text NOT NULL,
  input_token text NOT NULL,
  output_token text NOT NULL,
  attempts bigint NOT NULL,
  outcome text NOT NULL,
  outcome_count bigint NOT NULL,
  probe_input_atoms bigint[] NOT NULL,
  PRIMARY KEY (result_id, pool_id, input_token, output_token, outcome)
);

CREATE OR REPLACE VIEW research.v_policy_evidence AS
SELECT p.*, e.evidence_kind, e.status, e.scope_label, e.quote_symbol, e.quote_decimals,
  round(100::numeric * p.evaluable_candidates / NULLIF(p.candidates_evaluated, 0), 2) AS evaluable_share_pct,
  p.estimated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals) AS estimated_net_profit_quote,
  p.simulated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals) AS simulated_net_profit_quote,
  p.realized_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals) AS realized_net_profit_quote
FROM research.policy_results p JOIN research.experiments e USING (result_id);
CREATE OR REPLACE VIEW research.v_shape_evidence AS
SELECT s.*, e.evidence_kind, e.scope_label, e.quote_symbol, e.quote_decimals,
  round(100::numeric * s.evaluable_candidates / NULLIF(s.candidates_evaluated, 0), 2) AS evaluable_share_pct,
  s.estimated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals) AS estimated_net_profit_quote,
  s.simulated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals) AS simulated_net_profit_quote,
  s.realized_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals) AS realized_net_profit_quote
FROM research.shape_results s JOIN research.experiments e USING (result_id);
CREATE OR REPLACE VIEW research.v_policy_rejections AS
SELECT r.*, e.scope_label, e.quote_symbol FROM research.policy_rejections r JOIN research.experiments e USING (result_id);
CREATE OR REPLACE VIEW research.v_probe_evidence AS
SELECT p.*, e.seed_loaded_at, e.scope_label FROM research.pool_direction_probes p JOIN research.experiments e USING (result_id);

REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA research TO research_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA research TO research_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA research GRANT SELECT ON TABLES TO research_reader;
