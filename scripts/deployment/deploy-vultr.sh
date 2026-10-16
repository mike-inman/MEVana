#!/bin/bash
set -e

echo "🚀 MEVana Automated Deployment"
echo "=============================="
echo "Target: Vultr High Frequency Compute - NYC"
echo "Timestamp: $(date)"
echo ""

# Check if running on cloud instance (Vultr detection)
if [ ! -f /etc/cloud/cloud.cfg ] && [ ! -f /sys/class/dmi/id/product_name ] || ! grep -qi "vultr" /sys/class/dmi/id/product_name 2>/dev/null; then
    echo "⚠️  Warning: Not running on Vultr cloud instance"
    echo "This script is optimized for Vultr High Frequency Compute"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Deployment cancelled."
        exit 1
    fi
fi

# System information
echo "📊 System Information:"
echo "OS: $(lsb_release -d | cut -f2 2>/dev/null || cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2)"
echo "Kernel: $(uname -r)"
echo "CPU Cores: $(nproc)"
echo "RAM: $(free -h | grep Mem | awk '{print $2}')"
echo "Disk: $(df -h / | tail -1 | awk '{print $2}')"
echo ""

# Update system
echo "📦 Updating system packages..."
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get upgrade -y -qq

# Install essential dependencies
echo "🔧 Installing dependencies..."
apt-get install -y -qq \
    docker.io \
    docker-compose \
    git \
    curl \
    wget \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    gnupg \
    lsb-release \
    htop \
    net-tools \
    jq \
    bc

# Start and enable Docker
echo "🐳 Setting up Docker..."
systemctl start docker
systemctl enable docker
usermod -aG docker $USER

# Install Rust (if not present)
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    export PATH="$HOME/.cargo/bin:$PATH"
else
    echo "✅ Rust already installed: $(rustc --version)"
fi

# Clone/update MEVana repository
REPO_DIR="/opt/mevana"
if [ -d "$REPO_DIR" ]; then
    echo "📁 Updating existing MEVana repository..."
    cd $REPO_DIR
    git fetch origin
    git reset --hard origin/main
    git clean -fd
else
    echo "📥 Cloning MEVana repository..."
    git clone https://github.com/mike-inman/MEVana.git $REPO_DIR
    cd $REPO_DIR
fi

# Set permissions
chown -R $USER:$USER $REPO_DIR
chmod +x $REPO_DIR/tools/*.sh
chmod +x $REPO_DIR/*.sh

# Build MEVana with optimizations
echo "🏗️  Building MEVana (optimized release)..."
cd $REPO_DIR
export CARGO_TARGET_DIR="./target"

# Clean previous build
cargo clean

# Build with release optimizations
cargo build --release --workspace

# Verify critical binaries were built
REQUIRED_BINARIES=(
    "target/release/whirlpool-live-stream"
    "target/release/arb-engine"
    "target/release/executor-service"
)

echo "✅ Verifying build artifacts..."
for binary in "${REQUIRED_BINARIES[@]}"; do
    if [ -f "$binary" ]; then
        echo "  ✓ $binary"
        # Set executable permissions
        chmod +x "$binary"
    else
        echo "  ❌ Missing: $binary"
        echo "Build verification failed!"
        exit 1
    fi
done

# Create necessary directories
echo "📁 Creating service directories..."
mkdir -p logs pids data config prometheus_data grafana_data

# Set up environment configuration
echo "⚙️  Setting up environment configuration..."
cat > .env << 'EOF'
# Shyft.to Configuration
ENDPOINT=https://grpc.ny.shyft.to
X_TOKEN=<SHYFT_X_TOKEN>

# Kafka Configuration  
KAFKA_BROKER=localhost:29092

# Arbitrage Engine Configuration
ARB_MIN_PROFIT_BPS=25
ARB_HTTP_PORT=8081
HOTPATH_LOG=true

# Rust Configuration
RUST_LOG=info
RUST_BACKTRACE=1
EOF

# Start infrastructure services
echo "🐳 Starting infrastructure services..."
cd infrastructure

# Create custom docker-compose for production
cat > docker-compose.prod.yml << 'EOF'
version: '3.8'
services:
  zookeeper:
    image: confluentinc/cp-zookeeper:7.4.0
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000
    restart: unless-stopped
    
  kafka:
    image: confluentinc/cp-kafka:7.4.0
    depends_on:
      - zookeeper
    ports:
      - "29092:29092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:9092,PLAINTEXT_HOST://localhost:29092
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
      KAFKA_TRANSACTION_STATE_LOG_MIN_ISR: 1
      KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR: 1
      KAFKA_AUTO_CREATE_TOPICS_ENABLE: true
      KAFKA_NUM_PARTITIONS: 12
      KAFKA_DEFAULT_REPLICATION_FACTOR: 1
    restart: unless-stopped
    
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes
    restart: unless-stopped
    
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: mevdb
      POSTGRES_USER: mevana
      POSTGRES_PASSWORD: mevana123
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped
    
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ../prometheus_data:/prometheus
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
      - '--storage.tsdb.retention.time=200h'
      - '--web.enable-lifecycle'
    restart: unless-stopped
    
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: <ADMIN_PASSWORD>
    volumes:
      - ../grafana_data:/var/lib/grafana
    restart: unless-stopped

volumes:
  postgres_data:
EOF

# Start infrastructure with production config
docker-compose -f docker-compose.prod.yml up -d

# Wait for services to start
echo "⏳ Waiting for infrastructure services to start..."
sleep 45

# Verify infrastructure
echo "✅ Verifying infrastructure services..."
docker-compose -f docker-compose.prod.yml ps

# Check Kafka is ready
echo "🔍 Testing Kafka connectivity..."
timeout 30 bash -c 'while ! echo "exit" | nc localhost 29092; do sleep 1; done'
if [ $? -eq 0 ]; then
    echo "  ✓ Kafka is ready"
else
    echo "  ❌ Kafka connection failed"
    exit 1
fi

# Check Redis is ready
echo "🔍 Testing Redis connectivity..."
timeout 30 bash -c 'while ! redis-cli ping; do sleep 1; done'
if [ $? -eq 0 ]; then
    echo "  ✓ Redis is ready"
else
    echo "  ❌ Redis connection failed"
    exit 1
fi

# Go back to main directory
cd ..

echo ""
echo "🎉 MEVana Deployment Complete!"
echo "=============================="
echo "✅ System updated and dependencies installed"
echo "✅ MEVana built successfully in release mode"
echo "✅ Infrastructure services running:"
echo "   - Kafka: localhost:29092"
echo "   - Redis: localhost:6379"  
echo "   - PostgreSQL: localhost:5432"
echo "   - Prometheus: http://$(curl -s ipinfo.io/ip):9090"
echo "   - Grafana: http://$(curl -s ipinfo.io/ip):3000 (admin/<ADMIN_PASSWORD>)"
echo ""
echo "📋 Next Steps:"
echo "1. Run latency test: ./scripts/testing/latency_test.sh"
echo "2. Start services: ./start_services.sh"
echo "3. Monitor performance: ./monitor.sh"
echo "4. Check health: ./health_check.sh"
echo ""
echo "🚨 Critical Files:"
echo "   - Service logs: logs/"
echo "   - Process IDs: pids/"
echo "   - Configuration: .env"
echo ""
echo "⚡ Ready for MEV trading deployment!"
echo "Run './scripts/testing/latency_test.sh' to verify network performance."
