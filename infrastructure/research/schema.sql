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
  trim_scale(p.estimated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals)) AS estimated_net_profit_quote,
  trim_scale(p.simulated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals)) AS simulated_net_profit_quote,
  trim_scale(p.realized_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals)) AS realized_net_profit_quote
FROM research.policy_results p JOIN research.experiments e USING (result_id);
CREATE OR REPLACE VIEW research.v_shape_evidence AS
SELECT s.*, e.evidence_kind, e.scope_label, e.quote_symbol, e.quote_decimals,
  round(100::numeric * s.evaluable_candidates / NULLIF(s.candidates_evaluated, 0), 2) AS evaluable_share_pct,
  trim_scale(s.estimated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals)) AS estimated_net_profit_quote,
  trim_scale(s.simulated_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals)) AS simulated_net_profit_quote,
  trim_scale(s.realized_net_profit_quote_atoms::numeric / power(10::numeric, e.quote_decimals)) AS realized_net_profit_quote
FROM research.shape_results s JOIN research.experiments e USING (result_id);
CREATE OR REPLACE VIEW research.v_policy_rejections AS
SELECT r.*, e.scope_label, e.quote_symbol FROM research.policy_rejections r JOIN research.experiments e USING (result_id);
CREATE OR REPLACE VIEW research.v_probe_evidence AS
SELECT p.*, e.seed_loaded_at, e.scope_label,
  CASE p.input_token
    WHEN 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' THEN 'USDC'
    WHEN 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB' THEN 'USDT'
    WHEN 'So11111111111111111111111111111111111111112' THEN 'SOL'
    ELSE p.input_token END AS input_symbol,
  CASE p.output_token
    WHEN 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' THEN 'USDC'
    WHEN 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB' THEN 'USDT'
    WHEN 'So11111111111111111111111111111111111111112' THEN 'SOL'
    ELSE p.output_token END AS output_symbol
FROM research.pool_direction_probes p JOIN research.experiments e USING (result_id);

REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA research TO research_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA research TO research_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA research GRANT SELECT ON TABLES TO research_reader;

-- 2026-10-31: Indicative pool mid-price survey. Run-scoped rows preserve provenance.
CREATE TABLE IF NOT EXISTS research.gap_runs (
  run_id text PRIMARY KEY,
  evidence_kind text NOT NULL DEFAULT 'Indicative' CHECK (evidence_kind = 'Indicative'),
  provider text NOT NULL,
  window_start_utc timestamptz NOT NULL,
  window_end_utc timestamptz NOT NULL,
  duration_seconds bigint NOT NULL CHECK (duration_seconds > 0 AND duration_seconds <= 3600),
  payload_bytes bigint NOT NULL CHECK (payload_bytes >= 0),
  updates bigint NOT NULL CHECK (updates >= 0),
  updates_by_venue jsonb NOT NULL,
  gaps bigint NOT NULL CHECK (gaps >= 0),
  stalls bigint NOT NULL CHECK (stalls >= 0),
  stop_reason text NOT NULL,
  allowlist_sha256 text NOT NULL,
  updates_sha256 text NOT NULL,
  loaded_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS research.gap_pools (
  run_id text NOT NULL REFERENCES research.gap_runs(run_id) ON DELETE CASCADE,
  pool_id text NOT NULL,
  evidence_kind text NOT NULL DEFAULT 'Indicative' CHECK (evidence_kind = 'Indicative'),
  venue text NOT NULL,
  token_pair text NOT NULL,
  token_group text NOT NULL,
  mint0 text NOT NULL,
  mint1 text NOT NULL,
  decimals0 integer NOT NULL,
  decimals1 integer NOT NULL,
  fee_bps_micro bigint,
  base_fee_bps_micro bigint,
  fee_source text NOT NULL,
  variable_fee_modeled boolean NOT NULL,
  PRIMARY KEY (run_id,pool_id)
);
CREATE TABLE IF NOT EXISTS research.gap_minute_stats (
  run_id text NOT NULL REFERENCES research.gap_runs(run_id) ON DELETE CASCADE,
  minute_utc timestamptz NOT NULL,
  token_pair text NOT NULL,
  pool_a text NOT NULL,
  pool_b text NOT NULL,
  evidence_kind text NOT NULL DEFAULT 'Indicative' CHECK (evidence_kind = 'Indicative'),
  token_group text NOT NULL,
  venue_a text NOT NULL,
  venue_b text NOT NULL,
  observation_count bigint NOT NULL,
  known_fee_observation_count bigint NOT NULL CHECK (known_fee_observation_count >= 0 AND known_fee_observation_count <= observation_count),
  median_gross_bps_micro bigint,
  maximum_gross_bps_micro bigint,
  median_net_bps_micro bigint,
  maximum_net_bps_micro bigint,
  excluded_stale_count bigint NOT NULL,
  PRIMARY KEY (run_id,minute_utc,token_pair,pool_a,pool_b)
);
CREATE TABLE IF NOT EXISTS research.gap_events (
  run_id text NOT NULL REFERENCES research.gap_runs(run_id) ON DELETE CASCADE,
  token_pair text NOT NULL,
  pool_a text NOT NULL,
  pool_b text NOT NULL,
  threshold_bps integer NOT NULL CHECK (threshold_bps IN (5,10,25,50)),
  start_slot bigint NOT NULL,
  evidence_kind text NOT NULL DEFAULT 'Indicative' CHECK (evidence_kind = 'Indicative'),
  token_group text NOT NULL,
  venue_a text NOT NULL,
  venue_b text NOT NULL,
  start_time_utc timestamptz,
  duration_slots bigint NOT NULL,
  peak_net_bps_micro bigint NOT NULL,
  volatility_bucket text,
  after_sharp_move boolean NOT NULL,
  open_at_end boolean NOT NULL,
  PRIMARY KEY (run_id,token_pair,pool_a,pool_b,threshold_bps,start_slot)
);
CREATE TABLE IF NOT EXISTS research.gap_gate_results (
  run_set_id text NOT NULL,
  token_pair text NOT NULL,
  venue_combination text NOT NULL,
  evidence_kind text NOT NULL DEFAULT 'Indicative' CHECK (evidence_kind = 'Indicative'),
  result text NOT NULL CHECK (result IN ('advance','fail','too_little_data')),
  runs integer NOT NULL,
  events_per_hour_10bps_micro bigint,
  median_duration_slots bigint,
  stale_exclusion_percent_micro bigint,
  all_fees_known boolean NOT NULL,
  PRIMARY KEY (run_set_id,token_pair,venue_combination)
);
CREATE TABLE IF NOT EXISTS research.gap_observation_buckets (
  run_id text NOT NULL REFERENCES research.gap_runs(run_id) ON DELETE CASCADE,
  token_pair text NOT NULL,
  venue_combination text NOT NULL,
  token_group text NOT NULL,
  volatility_bucket text NOT NULL,
  after_sharp_move boolean NOT NULL,
  evidence_kind text NOT NULL DEFAULT 'Indicative' CHECK (evidence_kind = 'Indicative'),
  observations bigint NOT NULL CHECK (observations >= 0),
  PRIMARY KEY (run_id,token_pair,venue_combination,volatility_bucket,after_sharp_move)
);
GRANT SELECT ON ALL TABLES IN SCHEMA research TO research_reader;
