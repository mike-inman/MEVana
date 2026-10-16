#!/bin/bash
set -e

SERVER_HOST="149.28.56.125"
SERVER_USER="root"
DEPLOYMENT_PATH="/opt/MEVana"

echo "🚀 Direct MEVana Deployment to Production Server"
echo "================================================"

# Create deployment package
echo "📦 Creating deployment package..."
tar --exclude='.git' --exclude='target' --exclude='*.tar.gz' --exclude='_archive' -czf mevana-deploy.tar.gz .

# Upload to server
echo "⬆️ Uploading to server..."
scp mevana-deploy.tar.gz $SERVER_USER@$SERVER_HOST:/tmp/

# Deploy on server
echo "🔧 Deploying on server..."
ssh -T $SERVER_USER@$SERVER_HOST << 'ENDSSH'
set -e

echo "🛑 Stopping existing services..."
pkill -f "arb-engine" || true
pkill -f "whirlpool-live-stream" || true
pkill -f "hotpath_arb_engine" || true
pkill -f "multi-geyser-streams" || true
sleep 5

echo "📦 Extracting new code..."
cd /opt/MEVana
tar -xzf /tmp/mevana-deploy.tar.gz

echo "🔨 Building hotpath_arb_engine with Redis integration..."
export PATH="$HOME/.cargo/bin:$PATH"
cargo clean
cargo build --release -p hotpath_arb_engine

echo "🚀 Starting hotpath_arb_engine (Pure HotPath + Redis Metrics)..."
nohup bash -c '
export GEYSER_ENDPOINT="https://grpc.ny.shyft.to"
export X_TOKEN="<SHYFT_X_TOKEN>"
export METRICS_PORT=8088
export RUST_LOG=info
export ORDERBOOK_HISTORY_ENABLED=false
export REDIS_URL="redis://localhost:6379"
./target/release/hotpath_arb_engine
' > logs/hotpath_arb_engine.log 2>&1 &

echo "⏳ Waiting for service to start..."
sleep 15

echo "🔍 Health check..."
if curl -f http://localhost:8088/metrics >/dev/null 2>&1; then
    echo "✅ hotpath_arb_engine is healthy!"
    echo "🎯 Metrics: http://149.28.56.125:8088/metrics"
else
    echo "❌ Health check failed"
    tail -20 logs/hotpath_arb_engine.log
    exit 1
fi

echo "🎉 MAD MAX MEV DEPLOYMENT COMPLETE!"
ENDSSH

echo "✅ Direct deployment completed successfully!"
echo "📊 HotPath Engine: http://$SERVER_HOST:8088/metrics"