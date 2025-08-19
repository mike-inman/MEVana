#!/bin/bash

set -e

echo "🚀 Starting Live Arbitrage Engine Test"
echo "======================================"

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker Desktop."
    exit 1
fi

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env file..."
    cat > .env << EOF
# Kafka Configuration
KAFKA_BROKER=localhost:9092

# Database Configuration  
DATABASE_URL=postgresql://mev:mevpassword@localhost:5432/mevdb

# Arbitrage Engine Configuration
SWAP_TOPICS=raydium-swaps,orca-swaps
ARBITRAGE_TOPIC=arb-signals
PRICE_DIFF_THRESHOLD=0.005

# DEX Stream Configuration (optional - for real data)
ENDPOINT=https://api.mainnet-beta.solana.com
X_TOKEN=your_token_here
EOF
    echo "✅ .env file created. Please update X_TOKEN if you have one."
fi

# Start infrastructure services
echo "🔧 Starting infrastructure services..."
docker-compose up -d zookeeper kafka postgres

# Wait for services to be ready
echo "⏳ Waiting for services to be ready..."
sleep 10

# Run database migrations
echo "🗄️ Running database migrations..."
docker-compose exec postgres psql -U mev -d mevdb -c "
CREATE TABLE IF NOT EXISTS arb_opportunities (
    id SERIAL PRIMARY KEY,
    source_dex TEXT NOT NULL,
    target_dex TEXT NOT NULL,
    token_pair TEXT NOT NULL,
    profit_estimate DOUBLE PRECISION NOT NULL,
    raw_data JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_arb_opportunities_token_pair ON arb_opportunities(token_pair);
CREATE INDEX IF NOT EXISTS idx_arb_opportunities_created_at ON arb_opportunities(created_at);
"

# Start DEX streams (optional - for real data)
echo "📡 Starting DEX streams..."
docker-compose up -d dex-stream-raydium dex-stream-orca

# Wait a bit for streams to start
sleep 5

# Start arbitrage engine
echo "🧠 Starting arbitrage engine..."
docker-compose up -d arbitrage-engine

# Wait for arbitrage engine to start
sleep 5

# Show logs
echo "📋 Showing arbitrage engine logs..."
docker-compose logs -f arbitrage-engine &

# Show available services
echo ""
echo "🌐 Available Services:"
echo "  - Kafka UI: http://localhost:8080"
echo "  - Adminer (DB): http://localhost:8081"
echo "  - Arbitrage Engine Metrics: http://localhost:9094"
echo "  - Raydium Stream Metrics: http://localhost:9090"
echo "  - Orca Stream Metrics: http://localhost:9091"
echo ""

echo "✅ Live test environment is ready!"
echo "Press Ctrl+C to stop all services"
echo ""

# Keep script running and handle cleanup
trap 'echo "🛑 Stopping services..."; docker-compose down; exit 0' INT

# Wait for user to stop
wait 