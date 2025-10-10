-- View exposing latest preflight analytics per opportunity
CREATE OR REPLACE VIEW realtime_preflight_metrics AS
SELECT
    ao.timestamp,
    ao.opportunity_id,
    ao.token_pair,
    ao.venue_sell,
    ao.venue_buy,
    ao.profit_bps,
    ao.amount_in,
    ao.expected_amount_out,
    er.status,
    er.estimated_profit_lamports,
    er.estimated_bundle_fees_lamports,
    er.estimated_priority_fee_lamports,
    er.estimated_cu_cost_lamports,
    er.flash_loan_fee_lamports,
    er.min_profit_threshold_lamports,
    er.error_message,
    er.executed_at,
    er.simulated_token_deltas,
    (COALESCE(er.estimated_bundle_fees_lamports, 0)
     + COALESCE(er.estimated_priority_fee_lamports, 0)
     + COALESCE(er.estimated_cu_cost_lamports, 0)
     + COALESCE(er.flash_loan_fee_lamports, 0)) AS total_estimated_cost_lamports,
    (COALESCE(er.estimated_profit_lamports, 0)::numeric / 1e9) AS estimated_profit_sol,
    (COALESCE(er.min_profit_threshold_lamports, 0)::numeric / 1e9) AS min_profit_threshold_sol
FROM arbitrage_opportunities ao
LEFT JOIN LATERAL (
    SELECT er_inner.*
    FROM execution_results er_inner
    WHERE er_inner.opportunity_id::text = ao.opportunity_id
    ORDER BY er_inner.executed_at DESC
    LIMIT 1
) er ON TRUE;
