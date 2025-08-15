-- Migration: Add streaming services and network configuration tables
-- Date: 2024-01-XX
-- Description: Support for streaming service management and network switching

-- Create streaming_services table
CREATE TABLE IF NOT EXISTS streaming_services (
    id SERIAL PRIMARY KEY,
    service_id VARCHAR(100) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(50) DEFAULT 'stopped' CHECK (status IN ('running', 'stopped', 'error', 'starting', 'stopping')),
    enabled BOOLEAN DEFAULT false,
    data_source VARCHAR(100) NOT NULL,
    kafka_topic VARCHAR(100),
    docker_service VARCHAR(100),
    network VARCHAR(20) DEFAULT 'mainnet' CHECK (network IN ('mainnet', 'testnet')),
    metrics JSONB DEFAULT '{}',
    last_update TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create network_config table
CREATE TABLE IF NOT EXISTS network_config (
    id SERIAL PRIMARY KEY,
    network_id VARCHAR(20) UNIQUE NOT NULL CHECK (network_id IN ('mainnet', 'testnet')),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT false,
    rpc_endpoint VARCHAR(255),
    ws_endpoint VARCHAR(255),
    geyser_endpoint VARCHAR(255),
    last_switch TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create service_metrics table for historical metrics
CREATE TABLE IF NOT EXISTS service_metrics (
    id SERIAL PRIMARY KEY,
    service_id VARCHAR(100) NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    messages_per_second DECIMAL(10,2) DEFAULT 0,
    total_messages BIGINT DEFAULT 0,
    error_rate DECIMAL(5,4) DEFAULT 0,
    uptime_seconds INTEGER DEFAULT 0,
    memory_usage_mb INTEGER,
    cpu_usage_percent DECIMAL(5,2),
    network VARCHAR(20) DEFAULT 'mainnet',
    FOREIGN KEY (service_id) REFERENCES streaming_services(service_id) ON DELETE CASCADE
);

-- Create service_logs table for service events
CREATE TABLE IF NOT EXISTS service_logs (
    id SERIAL PRIMARY KEY,
    service_id VARCHAR(100) NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    level VARCHAR(20) NOT NULL CHECK (level IN ('INFO', 'WARN', 'ERROR', 'DEBUG')),
    message TEXT NOT NULL,
    details JSONB,
    network VARCHAR(20) DEFAULT 'mainnet',
    FOREIGN KEY (service_id) REFERENCES streaming_services(service_id) ON DELETE CASCADE
);

-- Add data_source and network columns to transactions table if they don't exist
DO $$ 
BEGIN
    -- Add data_source column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'transactions' AND column_name = 'data_source') THEN
        ALTER TABLE transactions ADD COLUMN data_source VARCHAR(100);
    END IF;
    
    -- Add network column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'transactions' AND column_name = 'network') THEN
        ALTER TABLE transactions ADD COLUMN network VARCHAR(20) DEFAULT 'mainnet';
    END IF;
END $$;

-- Add data_source and network columns to swap_events table if they don't exist
DO $$ 
BEGIN
    -- Add data_source column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'swap_events' AND column_name = 'data_source') THEN
        ALTER TABLE swap_events ADD COLUMN data_source VARCHAR(100);
    END IF;
    
    -- Add network column if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'swap_events' AND column_name = 'network') THEN
        ALTER TABLE swap_events ADD COLUMN network VARCHAR(20) DEFAULT 'mainnet';
    END IF;
END $$;

-- Insert default network configurations
INSERT INTO network_config (network_id, name, description, is_active, rpc_endpoint, ws_endpoint, geyser_endpoint) 
VALUES 
    ('mainnet', 'Mainnet (Live Trading)', 'Production Solana mainnet with real funds', true, 
     'https://api.mainnet-beta.solana.com', 'wss://api.mainnet-beta.solana.com', 'https://geyser.mainnet-beta.solana.com'),
    ('testnet', 'Testnet (Development)', 'Solana testnet for development and testing', false,
     'https://api.testnet.solana.com', 'wss://api.testnet.solana.com', 'https://geyser.testnet.solana.com')
ON CONFLICT (network_id) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    rpc_endpoint = EXCLUDED.rpc_endpoint,
    ws_endpoint = EXCLUDED.ws_endpoint,
    geyser_endpoint = EXCLUDED.geyser_endpoint,
    updated_at = CURRENT_TIMESTAMP;

-- Insert default streaming services
INSERT INTO streaming_services (service_id, name, description, data_source, kafka_topic, docker_service, network) 
VALUES 
    ('orca-stream', 'Orca Stream', 'Real-time Orca DEX transaction streaming', 'orca-dex-stream', 'orca.transactions', 'orca-stream', 'mainnet'),
    ('raydium-stream', 'Raydium Stream', 'Real-time Raydium DEX transaction streaming', 'raydium-dex-stream', 'raydium.transactions', 'raydium-stream', 'mainnet'),
    ('whale-tracking', 'Whale Tracking', 'Large transaction and whale wallet monitoring', 'whale-tracking-service', 'whale.alerts', 'whale-tracking-service', 'mainnet'),
    ('pool-volume', 'Pool Volume', 'DEX pool volume and liquidity tracking', 'pool-volume-service', 'pool.volume', 'pool-volume-service', 'mainnet'),
    ('address-enrichment', 'Address Enrichment', 'Solana address metadata enrichment', 'address-enrichment-service', 'address.enriched', 'address-enrichment-service', 'mainnet')
ON CONFLICT (service_id) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    data_source = EXCLUDED.data_source,
    kafka_topic = EXCLUDED.kafka_topic,
    docker_service = EXCLUDED.docker_service,
    network = EXCLUDED.network,
    updated_at = CURRENT_TIMESTAMP;

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_streaming_services_status ON streaming_services(status);
CREATE INDEX IF NOT EXISTS idx_streaming_services_network ON streaming_services(network);
CREATE INDEX IF NOT EXISTS idx_streaming_services_enabled ON streaming_services(enabled);
CREATE INDEX IF NOT EXISTS idx_service_metrics_service_id ON service_metrics(service_id);
CREATE INDEX IF NOT EXISTS idx_service_metrics_timestamp ON service_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_service_logs_service_id ON service_logs(service_id);
CREATE INDEX IF NOT EXISTS idx_service_logs_timestamp ON service_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_service_logs_level ON service_logs(level);
CREATE INDEX IF NOT EXISTS idx_transactions_data_source ON transactions(data_source);
CREATE INDEX IF NOT EXISTS idx_transactions_network ON transactions(network);
CREATE INDEX IF NOT EXISTS idx_swap_events_data_source ON swap_events(data_source);
CREATE INDEX IF NOT EXISTS idx_swap_events_network ON swap_events(network);

-- Create views for easier querying
CREATE OR REPLACE VIEW active_services AS
SELECT 
    ss.service_id,
    ss.name,
    ss.description,
    ss.status,
    ss.data_source,
    ss.network,
    ss.metrics,
    ss.last_update,
    nc.name as network_name,
    nc.rpc_endpoint,
    nc.ws_endpoint
FROM streaming_services ss
LEFT JOIN network_config nc ON ss.network = nc.network_id
WHERE ss.enabled = true;

CREATE OR REPLACE VIEW service_performance AS
SELECT 
    ss.service_id,
    ss.name,
    ss.data_source,
    ss.network,
    sm.messages_per_second,
    sm.total_messages,
    sm.error_rate,
    sm.uptime_seconds,
    sm.memory_usage_mb,
    sm.cpu_usage_percent,
    sm.timestamp
FROM streaming_services ss
LEFT JOIN service_metrics sm ON ss.service_id = sm.service_id
WHERE sm.timestamp >= CURRENT_TIMESTAMP - INTERVAL '1 hour'
ORDER BY sm.timestamp DESC;

-- Create functions for service management
CREATE OR REPLACE FUNCTION update_service_status(
    p_service_id VARCHAR(100),
    p_status VARCHAR(50),
    p_metrics JSONB DEFAULT NULL
)
RETURNS VOID AS $$
BEGIN
    UPDATE streaming_services 
    SET 
        status = p_status,
        metrics = COALESCE(p_metrics, metrics),
        last_update = CURRENT_TIMESTAMP,
        updated_at = CURRENT_TIMESTAMP
    WHERE service_id = p_service_id;
    
    -- Insert metrics record if provided
    IF p_metrics IS NOT NULL THEN
        INSERT INTO service_metrics (
            service_id, 
            messages_per_second, 
            total_messages, 
            error_rate, 
            uptime_seconds,
            memory_usage_mb,
            cpu_usage_percent,
            network
        ) VALUES (
            p_service_id,
            (p_metrics->>'messagesPerSecond')::DECIMAL,
            (p_metrics->>'totalMessages')::BIGINT,
            (p_metrics->>'errorRate')::DECIMAL,
            (p_metrics->>'uptimeSeconds')::INTEGER,
            (p_metrics->>'memoryUsageMb')::INTEGER,
            (p_metrics->>'cpuUsagePercent')::DECIMAL,
            (SELECT network FROM streaming_services WHERE service_id = p_service_id)
        );
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION log_service_event(
    p_service_id VARCHAR(100),
    p_level VARCHAR(20),
    p_message TEXT,
    p_details JSONB DEFAULT NULL
)
RETURNS VOID AS $$
BEGIN
    INSERT INTO service_logs (service_id, level, message, details, network)
    VALUES (
        p_service_id,
        p_level,
        p_message,
        p_details,
        (SELECT network FROM streaming_services WHERE service_id = p_service_id)
    );
END;
$$ LANGUAGE plpgsql;

-- Create trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_streaming_services_updated_at
    BEFORE UPDATE ON streaming_services
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_network_config_updated_at
    BEFORE UPDATE ON network_config
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Grant permissions (adjust as needed for your setup)
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO your_app_user;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO your_app_user; 