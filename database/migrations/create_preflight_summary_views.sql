-- Hourly and route-level preflight analytics views
CREATE OR REPLACE VIEW preflight_hourly_summary AS
SELECT
    date_trunc('hour', er.executed_at) AS hour_bucket,
    COUNT(*) FILTER (WHERE er.status = 'PreflightSuccess') AS preflight_passes,
    COUNT(*) FILTER (WHERE er.status <> 'PreflightSuccess') AS preflight_rejections,
    AVG(er.estimated_profit_lamports)::numeric / 1e9 AS avg_profit_sol,
    AVG((COALESCE(er.estimated_bundle_fees_lamports, 0)
        + COALESCE(er.estimated_priority_fee_lamports, 0)
        + COALESCE(er.estimated_cu_cost_lamports, 0)
        + COALESCE(er.flash_loan_fee_lamports, 0)))::numeric / 1e9 AS avg_cost_sol,
    AVG((er.estimated_profit_lamports
        - COALESCE(er.estimated_bundle_fees_lamports, 0)
        - COALESCE(er.estimated_priority_fee_lamports, 0)
        - COALESCE(er.estimated_cu_cost_lamports, 0)
        - COALESCE(er.flash_loan_fee_lamports, 0)))::numeric / 1e9 AS avg_net_sol
FROM execution_results er
WHERE er.executed_at >= NOW() - INTERVAL '48 hours'
GROUP BY hour_bucket
ORDER BY hour_bucket DESC;

CREATE OR REPLACE VIEW preflight_route_breakdown AS
SELECT
    ao.token_pair,
    ao.venue_sell || '→' || ao.venue_buy AS route,
    COUNT(*) FILTER (WHERE er.status = 'PreflightSuccess') AS passes,
    COUNT(*) FILTER (WHERE er.status <> 'PreflightSuccess') AS rejections,
    AVG((er.estimated_profit_lamports
        - COALESCE(er.estimated_bundle_fees_lamports, 0)
        - COALESCE(er.estimated_priority_fee_lamports, 0)
        - COALESCE(er.estimated_cu_cost_lamports, 0)
        - COALESCE(er.flash_loan_fee_lamports, 0)))::bigint AS avg_net_lamports,
    AVG(er.estimated_cu_cost_lamports)::bigint AS avg_cu_cost_lamports,
    AVG(er.estimated_priority_fee_lamports)::bigint AS avg_priority_fee_lamports
FROM arbitrage_opportunities ao
JOIN execution_results er ON er.opportunity_id::text = ao.opportunity_id
WHERE er.executed_at >= NOW() - INTERVAL '48 hours'
GROUP BY ao.token_pair, route
ORDER BY passes DESC, avg_net_lamports DESC;
