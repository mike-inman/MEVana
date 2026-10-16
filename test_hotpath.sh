#!/bin/bash
# Quick test script for HotPath engine

set -e

echo "🔍 Testing MEVana HotPath Engine..."
echo ""

# Check Docker services
echo "✅ Checking Docker services..."
if ! docker ps | grep -q "mevana-postgres-1"; then
    echo "❌ PostgreSQL not running. Start with: cd infrastructure && docker-compose up -d"
    exit 1
fi
echo "   PostgreSQL: Running ✓"
echo "   Redis: $(docker ps | grep redis && echo 'Running ✓' || echo 'Not running ✗')"
echo "   Kafka: $(docker ps | grep kafka-1 && echo 'Running ✓' || echo 'Not running ✗')"
echo ""

# Check database
echo "✅ Checking database connection..."
TABLE_COUNT=$(docker exec mevana-postgres-1 psql -U mev -d mevdb -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';")
echo "   Database tables: $TABLE_COUNT"
echo ""

# Build if needed
if [ ! -f "target/release/hotpath_arb_engine" ]; then
    echo "🔨 Building release binary (this may take a few minutes)..."
    DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --release -p hotpath_arb_engine
    echo "   Build complete ✓"
else
    echo "✅ Release binary already built"
fi
echo ""

# Run short test
echo "🚀 Running 30-second HotPath test (PAPER TRADING MODE)..."
echo "   Press Ctrl+C to stop early"
echo ""

PAPER_TRADING=true \
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="<SHYFT_X_TOKEN>" \
METRICS_PORT=8088 \
RUST_LOG=info \
timeout 30s ./target/release/hotpath_arb_engine || true

echo ""
echo "✅ Test complete!"
echo ""
echo "📊 To check metrics while running:"
echo "   curl localhost:8088/performance | jq ."
echo ""
echo "🔥 To run HotPath in production mode:"
echo "   Remove PAPER_TRADING=true and ensure wallet keys are configured"
