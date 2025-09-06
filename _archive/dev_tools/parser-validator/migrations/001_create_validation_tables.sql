-- Parser Validation Database Schema (PostgreSQL)
-- Creates tables for storing validation results, metrics, and reports

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Validation runs table - tracks individual validation sessions
CREATE TABLE validation_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ,
    config_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    target_dexes JSONB NOT NULL, -- JSON array of program IDs
    total_transactions INTEGER DEFAULT 0,
    successful_validations INTEGER DEFAULT 0,
    failed_validations INTEGER DEFAULT 0,
    average_accuracy DECIMAL(5,4) DEFAULT 0.0,
    average_completeness DECIMAL(5,4) DEFAULT 0.0,
    average_parse_time_us BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Individual transaction validation results
CREATE TABLE transaction_validations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    validation_run_id UUID NOT NULL,
    transaction_signature TEXT NOT NULL,
    program_id TEXT NOT NULL,
    instruction_name TEXT,
    
    -- Accuracy metrics
    overall_accuracy DECIMAL(5,4) NOT NULL,
    field_accuracies JSONB NOT NULL, -- JSON object
    price_deviation_bps INTEGER,
    amount_deviation_bps INTEGER,
    
    -- Completeness metrics
    completeness_ratio DECIMAL(5,4) NOT NULL,
    fields_parsed INTEGER NOT NULL,
    fields_expected INTEGER NOT NULL,
    missing_fields JSONB, -- JSON array
    
    -- Performance metrics
    parse_time_us BIGINT NOT NULL,
    validation_time_us BIGINT NOT NULL,
    
    -- Ground truth metadata
    ground_truth_confidence DECIMAL(5,4) NOT NULL,
    ground_truth_sources JSONB NOT NULL, -- JSON array
    
    -- Error counts
    error_count INTEGER DEFAULT 0,
    warning_count INTEGER DEFAULT 0,
    
    -- Timestamps
    block_time TIMESTAMPTZ NOT NULL,
    validation_time TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (validation_run_id) REFERENCES validation_runs(id) ON DELETE CASCADE
);

-- Validation errors table
CREATE TABLE validation_errors (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    validation_run_id UUID NOT NULL,
    transaction_validation_id UUID,
    transaction_signature TEXT NOT NULL,
    program_id TEXT NOT NULL,
    
    -- Error details
    category TEXT NOT NULL,
    field_name TEXT,
    message TEXT NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('Low', 'Medium', 'High', 'Critical')),
    error_details JSONB, -- JSON object
    
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (validation_run_id) REFERENCES validation_runs(id) ON DELETE CASCADE,
    FOREIGN KEY (transaction_validation_id) REFERENCES transaction_validations(id) ON DELETE CASCADE
);

-- Validation warnings table
CREATE TABLE validation_warnings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    validation_run_id UUID NOT NULL,
    transaction_validation_id UUID,
    transaction_signature TEXT NOT NULL,
    program_id TEXT NOT NULL,
    
    -- Warning details
    category TEXT NOT NULL,
    field_name TEXT,
    message TEXT NOT NULL,
    recommendation TEXT,
    
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (validation_run_id) REFERENCES validation_runs(id) ON DELETE CASCADE,
    FOREIGN KEY (transaction_validation_id) REFERENCES transaction_validations(id) ON DELETE CASCADE
);

-- DEX coverage reports - aggregated metrics per DEX
CREATE TABLE dex_coverage_reports (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    validation_run_id UUID NOT NULL,
    program_id TEXT NOT NULL,
    dex_name TEXT NOT NULL,
    
    -- Transaction coverage
    total_transactions INTEGER NOT NULL,
    successfully_parsed INTEGER NOT NULL,
    parse_success_rate DECIMAL(5,4) NOT NULL,
    
    -- Instruction coverage
    unique_instructions INTEGER NOT NULL,
    supported_instructions INTEGER NOT NULL,
    instruction_coverage_rate DECIMAL(5,4) NOT NULL,
    instruction_breakdown JSONB, -- JSON object
    
    -- Accuracy metrics
    average_accuracy DECIMAL(5,4) NOT NULL,
    min_accuracy DECIMAL(5,4) NOT NULL,
    max_accuracy DECIMAL(5,4) NOT NULL,
    accuracy_p95 DECIMAL(5,4) NOT NULL,
    
    -- Completeness metrics
    average_completeness DECIMAL(5,4) NOT NULL,
    min_completeness DECIMAL(5,4) NOT NULL,
    max_completeness DECIMAL(5,4) NOT NULL,
    completeness_p95 DECIMAL(5,4) NOT NULL,
    
    -- Performance metrics
    avg_parse_time_us BIGINT NOT NULL,
    min_parse_time_us BIGINT NOT NULL,
    max_parse_time_us BIGINT NOT NULL,
    p50_parse_time_us BIGINT NOT NULL,
    p95_parse_time_us BIGINT NOT NULL,
    p99_parse_time_us BIGINT NOT NULL,
    
    -- Error metrics
    total_errors INTEGER NOT NULL,
    total_warnings INTEGER NOT NULL,
    error_rate DECIMAL(5,4) NOT NULL,
    warning_rate DECIMAL(5,4) NOT NULL,
    error_breakdown JSONB, -- JSON object
    
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (validation_run_id) REFERENCES validation_runs(id) ON DELETE CASCADE
);

-- Performance benchmarks - latency and throughput testing
CREATE TABLE performance_benchmarks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    validation_run_id UUID NOT NULL,
    benchmark_type TEXT NOT NULL CHECK (benchmark_type IN ('latency', 'throughput', 'load_test')),
    program_id TEXT,
    
    -- Test configuration
    target_tps INTEGER,
    test_duration_seconds INTEGER,
    sample_size INTEGER,
    
    -- Latency metrics (microseconds)
    min_latency_us BIGINT,
    max_latency_us BIGINT,
    avg_latency_us BIGINT,
    p50_latency_us BIGINT,
    p95_latency_us BIGINT,
    p99_latency_us BIGINT,
    
    -- Throughput metrics
    achieved_tps DECIMAL(10,2),
    total_processed INTEGER,
    success_rate DECIMAL(5,4),
    error_rate DECIMAL(5,4),
    timeout_rate DECIMAL(5,4),
    
    -- Resource metrics
    peak_memory_mb DECIMAL(10,2),
    avg_memory_mb DECIMAL(10,2),
    cpu_utilization_percent DECIMAL(5,2),
    
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (validation_run_id) REFERENCES validation_runs(id) ON DELETE CASCADE
);

-- Ground truth source reliability - track API performance
CREATE TABLE ground_truth_reliability (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_name TEXT NOT NULL CHECK (source_name IN ('shyft', 'helius', 'solscan')),
    transaction_signature TEXT NOT NULL,
    
    -- Request metrics
    request_timestamp TIMESTAMPTZ NOT NULL,
    response_timestamp TIMESTAMPTZ,
    request_duration_ms INTEGER,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    
    -- Data quality
    data_completeness DECIMAL(5,4), -- 0.0-1.0
    confidence_score DECIMAL(5,4),  -- 0.0-1.0
    fields_provided INTEGER,
    
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Real-time metrics aggregates - for dashboard display
CREATE TABLE real_time_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_timestamp TIMESTAMPTZ NOT NULL,
    time_window_minutes INTEGER NOT NULL CHECK (time_window_minutes IN (1, 5, 15, 60)),
    
    -- Overall metrics
    transactions_processed INTEGER NOT NULL,
    overall_accuracy DECIMAL(5,4) NOT NULL,
    overall_completeness DECIMAL(5,4) NOT NULL,
    average_parse_time_us BIGINT NOT NULL,
    
    -- Per-DEX breakdown
    dex_metrics JSONB NOT NULL, -- JSON object
    
    -- Error rates
    error_rate DECIMAL(5,4) NOT NULL,
    warning_rate DECIMAL(5,4) NOT NULL,
    
    -- Performance
    p95_latency_us BIGINT NOT NULL,
    throughput_tps DECIMAL(10,2) NOT NULL,
    
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_transaction_validations_run_id ON transaction_validations(validation_run_id);
CREATE INDEX idx_transaction_validations_program_id ON transaction_validations(program_id);
CREATE INDEX idx_transaction_validations_signature ON transaction_validations(transaction_signature);
CREATE INDEX idx_transaction_validations_block_time ON transaction_validations(block_time);
CREATE INDEX idx_transaction_validations_accuracy ON transaction_validations(overall_accuracy);
CREATE INDEX idx_transaction_validations_completeness ON transaction_validations(completeness_ratio);

CREATE INDEX idx_validation_errors_run_id ON validation_errors(validation_run_id);
CREATE INDEX idx_validation_errors_program_id ON validation_errors(program_id);
CREATE INDEX idx_validation_errors_category ON validation_errors(category);
CREATE INDEX idx_validation_errors_severity ON validation_errors(severity);

CREATE INDEX idx_validation_warnings_run_id ON validation_warnings(validation_run_id);
CREATE INDEX idx_validation_warnings_program_id ON validation_warnings(program_id);
CREATE INDEX idx_validation_warnings_category ON validation_warnings(category);

CREATE INDEX idx_dex_coverage_run_id ON dex_coverage_reports(validation_run_id);
CREATE INDEX idx_dex_coverage_program_id ON dex_coverage_reports(program_id);
CREATE INDEX idx_dex_coverage_success_rate ON dex_coverage_reports(parse_success_rate);

CREATE INDEX idx_performance_benchmarks_run_id ON performance_benchmarks(validation_run_id);
CREATE INDEX idx_performance_benchmarks_type ON performance_benchmarks(benchmark_type);
CREATE INDEX idx_performance_benchmarks_program_id ON performance_benchmarks(program_id);

CREATE INDEX idx_ground_truth_reliability_source ON ground_truth_reliability(source_name);
CREATE INDEX idx_ground_truth_reliability_timestamp ON ground_truth_reliability(request_timestamp);
CREATE INDEX idx_ground_truth_reliability_success ON ground_truth_reliability(success);

CREATE INDEX idx_real_time_metrics_timestamp ON real_time_metrics(metric_timestamp);
CREATE INDEX idx_real_time_metrics_window ON real_time_metrics(time_window_minutes);

-- Views for common queries
CREATE VIEW validation_summary AS
SELECT 
    vr.id as run_id,
    vr.start_time,
    vr.end_time,
    vr.status,
    vr.total_transactions,
    vr.successful_validations,
    vr.failed_validations,
    CASE 
        WHEN vr.total_transactions > 0 
        THEN vr.successful_validations::DECIMAL / vr.total_transactions 
        ELSE 0 
    END as success_rate,
    vr.average_accuracy,
    vr.average_completeness,
    vr.average_parse_time_us,
    COUNT(DISTINCT tv.program_id) as unique_dexes,
    COUNT(ve.id) as total_errors,
    COUNT(vw.id) as total_warnings
FROM validation_runs vr
LEFT JOIN transaction_validations tv ON vr.id = tv.validation_run_id
LEFT JOIN validation_errors ve ON vr.id = ve.validation_run_id
LEFT JOIN validation_warnings vw ON vr.id = vw.validation_run_id
GROUP BY vr.id, vr.start_time, vr.end_time, vr.status, vr.total_transactions, 
         vr.successful_validations, vr.failed_validations, vr.average_accuracy, 
         vr.average_completeness, vr.average_parse_time_us;

CREATE VIEW dex_performance_summary AS
SELECT 
    program_id,
    dex_name,
    COUNT(*) as total_reports,
    AVG(parse_success_rate) as avg_success_rate,
    AVG(average_accuracy) as avg_accuracy,
    AVG(average_completeness) as avg_completeness,
    AVG(avg_parse_time_us) as avg_parse_time_us,
    AVG(error_rate) as avg_error_rate
FROM dex_coverage_reports
GROUP BY program_id, dex_name;

-- Trigger to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_validation_runs_updated_at 
    BEFORE UPDATE ON validation_runs 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();