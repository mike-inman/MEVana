#\!/bin/bash

# MEVana Clean Startup Script
# Prevents port/database conflicts

echo "🧹 Cleaning up any existing processes..."
pkill -f "cargo run -p hotpath_arb_engine" 2>/dev/null || true
pkill -f "cargo run -p system-orchestrator" 2>/dev/null || true

echo "🔍 Checking database status..."
docker exec postgres psql -U mev -d mevdb -c "SELECT COUNT(*) as opportunities, MAX(timestamp) as latest FROM arbitrage_opportunities;" 2>/dev/null || echo "❌ Database not accessible"

echo "🚀 Starting system-orchestrator with correct configuration..."
DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
RUST_LOG=info \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="efff2d23-97ae-4c5f-8b8b-9a6265001189" \
cargo run -p system-orchestrator

