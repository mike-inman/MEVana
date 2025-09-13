#!/bin/bash
set -e

SERVER_HOST="149.28.56.125"
SERVER_USER="root"
DEPLOYMENT_PATH="/opt/MEVana"

echo "🚀 Deploying Monitoring Service to Production Server"
echo "================================================="

# Create deployment package
echo "📦 Creating deployment package..."
tar --exclude='.git' --exclude='target' --exclude='*.tar.gz' --exclude='_archive' -czf mevana-monitoring-deploy.tar.gz .

# Upload to server
echo "⬆️ Uploading to server..."
scp mevana-monitoring-deploy.tar.gz $SERVER_USER@$SERVER_HOST:/tmp/

# Deploy on server
echo "🔧 Deploying monitoring service on server..."
ssh -T $SERVER_USER@$SERVER_HOST << 'ENDSSH'
set -e

echo "📦 Extracting new code..."
cd /opt/MEVana
tar -xzf /tmp/mevana-monitoring-deploy.tar.gz

echo "🔨 Building monitoring-service..."
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p monitoring-service

echo "🛑 Stopping any existing monitoring service..."
pkill -f "monitoring-service" || true
sleep 2

echo "🚀 Starting monitoring service on port 8090..."
nohup bash -c '
export MONITORING_PORT=8090
export REDIS_URL="redis://localhost:6379"
export DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb"
export RUST_LOG=info
./target/release/monitoring-service
' > logs/monitoring-service.log 2>&1 &

echo "⏳ Waiting for monitoring service to start..."
sleep 10

echo "🔍 Health check..."
if curl -f http://localhost:8090/health >/dev/null 2>&1; then
    echo "✅ Monitoring service is healthy!"
    echo "🎯 Health: http://149.28.56.125:8090/health"
    echo "📊 Live Stats: http://149.28.56.125:8090/api/arbitrage/live"
    echo "⚡ Performance: http://149.28.56.125:8090/api/arbitrage/performance"
    echo "🖥️  System Status: http://149.28.56.125:8090/api/system/status"
else
    echo "❌ Health check failed"
    tail -20 logs/monitoring-service.log
    exit 1
fi

echo "🎉 MONITORING SERVICE DEPLOYMENT COMPLETE!"
ENDSSH

echo "✅ Monitoring service deployment completed successfully!"
echo "🌐 Monitoring API: http://$SERVER_HOST:8090"