-- Smart Filter Configuration Management Schema
-- Created: 2025-09-11
-- Purpose: Track configuration changes and performance impact

-- Configuration history and state management
CREATE TABLE IF NOT EXISTS smart_filter_config_history (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    parameter_name VARCHAR(100) NOT NULL,
    old_value JSONB,
    new_value JSONB NOT NULL,
    applied_by VARCHAR(255) NOT NULL,
    performance_before JSONB,
    performance_after JSONB,
    
    CONSTRAINT valid_parameter_name CHECK (
        parameter_name IN (
            'min_net_profit_sol', 'min_profit_bps', 'max_profit_bps',
            'min_venue_success_rate', 'max_gas_budget_pct', 
            'max_concurrent_opportunities', 'competition_lookback_ms', 'learning_rate',
            'all_parameters'  -- For reset operations
        )
    )
);

-- Performance impact tracking
CREATE TABLE IF NOT EXISTS parameter_performance_impact (
    id SERIAL PRIMARY KEY,
    config_change_id INTEGER REFERENCES smart_filter_config_history(id),
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    opportunities_per_minute DECIMAL(8,2),
    success_rate DECIMAL(5,4),
    average_profit_bps DECIMAL(8,2),
    execution_latency_us BIGINT,
    sample_size INTEGER,
    measurement_period_minutes INTEGER DEFAULT 5
);

-- Smart filter performance metrics (for dashboard)
CREATE TABLE IF NOT EXISTS smart_filter_performance_snapshots (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    opportunities_detected INTEGER DEFAULT 0,
    opportunities_executed INTEGER DEFAULT 0,
    total_profit_captured_bps BIGINT DEFAULT 0,
    success_rate DECIMAL(5,4) DEFAULT 0,
    avg_processing_time_us INTEGER DEFAULT 0,
    
    -- Filter statistics
    total_opportunities INTEGER DEFAULT 0,
    passed_profit_filter INTEGER DEFAULT 0,
    passed_gas_filter INTEGER DEFAULT 0,
    passed_competition_filter INTEGER DEFAULT 0,
    rejected_low_profit INTEGER DEFAULT 0,
    rejected_high_gas INTEGER DEFAULT 0,
    rejected_suspicious INTEGER DEFAULT 0,
    rejected_low_success_rate INTEGER DEFAULT 0,
    rejected_competition INTEGER DEFAULT 0
);

-- Venue performance tracking for smart filter learning
CREATE TABLE IF NOT EXISTS venue_performance_history (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    venue_pair VARCHAR(200) NOT NULL,
    attempts INTEGER DEFAULT 0,
    successes INTEGER DEFAULT 0,
    total_profit DECIMAL(15,6) DEFAULT 0,
    avg_execution_time_us INTEGER DEFAULT 0
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_config_history_timestamp ON smart_filter_config_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_config_history_parameter ON smart_filter_config_history(parameter_name, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_performance_impact_timestamp ON parameter_performance_impact(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_performance_snapshots_timestamp ON smart_filter_performance_snapshots(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_venue_performance_timestamp ON venue_performance_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_venue_performance_pair ON venue_performance_history(venue_pair, timestamp DESC);

-- View for latest configuration
CREATE OR REPLACE VIEW current_smart_filter_config AS
SELECT DISTINCT ON (parameter_name)
    parameter_name,
    new_value,
    timestamp,
    applied_by
FROM smart_filter_config_history
ORDER BY parameter_name, timestamp DESC;

-- View for configuration change impact analysis
CREATE OR REPLACE VIEW config_change_impact_analysis AS
SELECT 
    c.id,
    c.timestamp as change_timestamp,
    c.parameter_name,
    c.old_value,
    c.new_value,
    c.applied_by,
    c.performance_before,
    c.performance_after,
    
    -- Calculate impact if both before/after data exists
    CASE 
        WHEN c.performance_before IS NOT NULL AND c.performance_after IS NOT NULL THEN
            (c.performance_after->>'opportunities_per_minute')::DECIMAL - 
            (c.performance_before->>'opportunities_per_minute')::DECIMAL
        ELSE NULL 
    END as opportunities_change,
    
    CASE 
        WHEN c.performance_before IS NOT NULL AND c.performance_after IS NOT NULL THEN
            (c.performance_after->>'success_rate')::DECIMAL - 
            (c.performance_before->>'success_rate')::DECIMAL
        ELSE NULL 
    END as success_rate_change,
    
    CASE 
        WHEN c.performance_before IS NOT NULL AND c.performance_after IS NOT NULL THEN
            (c.performance_after->>'average_profit_bps')::DECIMAL - 
            (c.performance_before->>'average_profit_bps')::DECIMAL
        ELSE NULL 
    END as profit_change
    
FROM smart_filter_config_history c
WHERE c.timestamp >= NOW() - INTERVAL '30 days'
ORDER BY c.timestamp DESC;

-- View for recent performance trends
CREATE OR REPLACE VIEW recent_performance_trends AS
SELECT 
    DATE_TRUNC('hour', timestamp) as hour_bucket,
    AVG(opportunities_detected) as avg_opportunities_detected,
    AVG(success_rate) as avg_success_rate,
    AVG(total_profit_captured_bps) as avg_profit_bps,
    AVG(avg_processing_time_us) as avg_processing_time_us,
    COUNT(*) as snapshot_count
FROM smart_filter_performance_snapshots
WHERE timestamp >= NOW() - INTERVAL '24 hours'
GROUP BY DATE_TRUNC('hour', timestamp)
ORDER BY hour_bucket DESC;

-- Function to record performance snapshot
CREATE OR REPLACE FUNCTION record_performance_snapshot(
    p_opportunities_detected INTEGER DEFAULT 0,
    p_opportunities_executed INTEGER DEFAULT 0,
    p_total_profit_captured_bps BIGINT DEFAULT 0,
    p_success_rate DECIMAL(5,4) DEFAULT 0,
    p_avg_processing_time_us INTEGER DEFAULT 0,
    p_total_opportunities INTEGER DEFAULT 0,
    p_passed_profit_filter INTEGER DEFAULT 0,
    p_passed_gas_filter INTEGER DEFAULT 0,
    p_passed_competition_filter INTEGER DEFAULT 0,
    p_rejected_low_profit INTEGER DEFAULT 0,
    p_rejected_high_gas INTEGER DEFAULT 0,
    p_rejected_suspicious INTEGER DEFAULT 0,
    p_rejected_low_success_rate INTEGER DEFAULT 0,
    p_rejected_competition INTEGER DEFAULT 0
) RETURNS INTEGER AS $$
DECLARE
    snapshot_id INTEGER;
BEGIN
    INSERT INTO smart_filter_performance_snapshots (
        opportunities_detected,
        opportunities_executed,
        total_profit_captured_bps,
        success_rate,
        avg_processing_time_us,
        total_opportunities,
        passed_profit_filter,
        passed_gas_filter,
        passed_competition_filter,
        rejected_low_profit,
        rejected_high_gas,
        rejected_suspicious,
        rejected_low_success_rate,
        rejected_competition
    ) VALUES (
        p_opportunities_detected,
        p_opportunities_executed,
        p_total_profit_captured_bps,
        p_success_rate,
        p_avg_processing_time_us,
        p_total_opportunities,
        p_passed_profit_filter,
        p_passed_gas_filter,
        p_passed_competition_filter,
        p_rejected_low_profit,
        p_rejected_high_gas,
        p_rejected_suspicious,
        p_rejected_low_success_rate,
        p_rejected_competition
    ) RETURNING id INTO snapshot_id;
    
    RETURN snapshot_id;
END;
$$ LANGUAGE plpgsql;

-- Function to update configuration performance impact
CREATE OR REPLACE FUNCTION update_config_performance_impact(
    p_config_change_id INTEGER,
    p_opportunities_per_minute DECIMAL(8,2),
    p_success_rate DECIMAL(5,4),
    p_average_profit_bps DECIMAL(8,2),
    p_execution_latency_us BIGINT,
    p_sample_size INTEGER DEFAULT 100
) RETURNS BOOLEAN AS $$
BEGIN
    INSERT INTO parameter_performance_impact (
        config_change_id,
        opportunities_per_minute,
        success_rate,
        average_profit_bps,
        execution_latency_us,
        sample_size
    ) VALUES (
        p_config_change_id,
        p_opportunities_per_minute,
        p_success_rate,
        p_average_profit_bps,
        p_execution_latency_us,
        p_sample_size
    );
    
    -- Also update the config history record with performance_after
    UPDATE smart_filter_config_history 
    SET performance_after = jsonb_build_object(
        'opportunities_per_minute', p_opportunities_per_minute,
        'success_rate', p_success_rate,
        'average_profit_bps', p_average_profit_bps,
        'execution_latency_us', p_execution_latency_us,
        'timestamp', NOW()
    )
    WHERE id = p_config_change_id;
    
    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- Data retention policy: Keep data for 90 days
CREATE OR REPLACE FUNCTION cleanup_old_performance_data() RETURNS BOOLEAN AS $$
BEGIN
    -- Clean up old performance snapshots (keep 30 days)
    DELETE FROM smart_filter_performance_snapshots 
    WHERE timestamp < NOW() - INTERVAL '30 days';
    
    -- Clean up old venue performance (keep 90 days)
    DELETE FROM venue_performance_history 
    WHERE timestamp < NOW() - INTERVAL '90 days';
    
    -- Clean up old performance impact data (keep 90 days) 
    DELETE FROM parameter_performance_impact 
    WHERE timestamp < NOW() - INTERVAL '90 days';
    
    -- Keep config history for longer (1 year)
    DELETE FROM smart_filter_config_history 
    WHERE timestamp < NOW() - INTERVAL '1 year';
    
    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- Schedule cleanup to run daily (requires pg_cron extension)
-- SELECT cron.schedule('cleanup-smart-filter-data', '0 2 * * *', 'SELECT cleanup_old_performance_data();');

-- Grant permissions (adjust user as needed)
-- GRANT SELECT, INSERT, UPDATE ON smart_filter_config_history TO mevana;
-- GRANT SELECT, INSERT, UPDATE ON parameter_performance_impact TO mevana;
-- GRANT SELECT, INSERT, UPDATE ON smart_filter_performance_snapshots TO mevana;
-- GRANT SELECT, INSERT, UPDATE ON venue_performance_history TO mevana;
-- GRANT USAGE ON SEQUENCE smart_filter_config_history_id_seq TO mevana;
-- GRANT USAGE ON SEQUENCE parameter_performance_impact_id_seq TO mevana;
-- GRANT USAGE ON SEQUENCE smart_filter_performance_snapshots_id_seq TO mevana;
-- GRANT USAGE ON SEQUENCE venue_performance_history_id_seq TO mevana;

-- Insert initial configuration record for tracking
-- INSERT INTO smart_filter_config_history (parameter_name, new_value, applied_by, performance_before)
-- VALUES ('all_parameters', '{"setup": "complete"}', 'system', '{"opportunities_per_minute": 0, "success_rate": 0, "average_profit_bps": 0, "execution_latency_us": 0, "timestamp": "2025-09-11T00:00:00Z"}'::jsonb)
-- ON CONFLICT DO NOTHING;