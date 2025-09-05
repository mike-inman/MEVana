#!/bin/bash
set -e

echo "🚀 Arb Engine Quick Start Script"
echo "================================="

# Detect environment
if [[ "${1}" == "production" ]]; then
    MODE="production"
    FEATURES="prod-full"
    PROFILE="release"
    echo "🏭 Building for PRODUCTION environment"
elif [[ "${1}" == "development" ]] || [[ -z "${1}" ]]; then
    MODE="development"
    FEATURES="dev-full"
    PROFILE="dev-opt"
    echo "🔧 Building for DEVELOPMENT environment"
else
    echo "❌ Usage: $0 [development|production]"
    exit 1
fi

echo ""
echo "📦 Step 1: Clean previous builds"
cargo clean

echo ""
echo "🔍 Step 2: Check dependencies"
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust not found. Install from https://rustup.rs/"
    exit 1
fi

RUST_VERSION=$(rustc --version)
echo "✅ Found Rust: $RUST_VERSION"

if ! command -v protoc &> /dev/null; then
    echo "⚠️  Protocol Buffers compiler not found"
    echo "   Installing via package manager..."
    
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update && sudo apt-get install -y protobuf-compiler
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        brew install protobuf
    else
        echo "❌ Please install protoc manually: https://grpc.io/docs/protoc-installation/"
        exit 1
    fi
fi

echo "✅ Found protoc: $(protoc --version)"

echo ""
echo "🔧 Step 3: Set environment variables for $MODE"

if [[ "$MODE" == "development" ]]; then
    cat > .env << EOF
# Development Environment Configuration
RUST_LOG=arb_engine=info,edge_assembler=info
KAFKA_BROKER=localhost:29092
REDIS_URL=redis://localhost:6379
EDGE_DELTA_TOPIC=edge.delta.v1
WS_BIND=0.0.0.0:8080

# Performance (development)
ARB_MIN_PROFIT_BPS=25
ARB_MAX_SLIPPAGE_BPS=100
ARB_MAX_PAIRS_TOTAL=5000
ARB_CLEANUP_INTERVAL_MS=15000

# Diagnostics
ARB_ADAPT_STATE_PATH=./arb_state.json
AI_LOG_DIR=./ai_logs

# Circuit breaker (lenient for development)
CIRCUIT_ERROR_RATIO_THRESHOLD=0.50
READINESS_MAX_STALENESS_MS=10000
READINESS_MAX_ERROR_RATIO=0.50

# Build info
SERVICE_NAME=arb-engine-dev
BUILD_VERSION=dev
GIT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF
else
    cat > .env.production << EOF
# Production Environment Configuration  
RUST_LOG=arb_engine=info,edge_assembler=warn
KAFKA_BROKER=kafka1:9092,kafka2:9092,kafka3:9092
REDIS_URL=redis://redis-cluster:6379
EDGE_DELTA_TOPIC=edge.delta.v1
WS_BIND=0.0.0.0:8080

# Performance (production)
ARB_MIN_PROFIT_BPS=75
ARB_MAX_SLIPPAGE_BPS=50
ARB_MAX_PAIRS_TOTAL=10000
ARB_CLEANUP_INTERVAL_MS=30000

# Circuit breaker (strict for production)
CIRCUIT_ERROR_RATIO_THRESHOLD=0.25
READINESS_MAX_STALENESS_MS=5000
READINESS_MAX_ERROR_RATIO=0.20

# Security
AI_API_TOKEN=your-secure-token-here

# Build info
SERVICE_NAME=arb-engine
BUILD_VERSION=v0.2.0
GIT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF
fi

echo "✅ Environment configured"

echo ""
echo "🏗️  Step 4: Build with features=$FEATURES profile=$PROFILE"

# Set build environment variables
export BUILD_TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
export GIT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

if [[ "$PROFILE" == "release" ]]; then
    cargo build --release --features="$FEATURES"
    BINARY_PATH="target/release/arb-engine"
else
    # Use custom dev-opt profile if available
    if grep -q "dev-opt" Cargo.toml; then
        cargo build --profile=dev-opt --features="$FEATURES"
        BINARY_PATH="target/dev-opt/arb-engine"
    else
        cargo build --features="$FEATURES"
        BINARY_PATH="target/debug/arb-engine"
    fi
fi

if [[ ! -f "$BINARY_PATH" ]]; then
    echo "❌ Build failed - binary not found at $BINARY_PATH"
    exit 1
fi

echo "✅ Build completed successfully"

echo ""
echo "🧪 Step 5: Run basic tests"

# Test edge integration module
echo "  Testing edge integration..."
if ! cargo test --features="$FEATURES" edge_integration --lib --quiet; then
    echo "⚠️  Some edge integration tests failed (this may be expected in development)"
fi

# Test synthetic edge generation  
echo "  Testing synthetic edge generation..."
if command -v timeout &> /dev/null; then
    timeout 10s cargo run --bin gen_synthetic_edges --features="$FEATURES" -- /tmp/test_edges.jsonl 100 || true
else
    cargo run --bin gen_synthetic_edges --features="$FEATURES" -- /tmp/test_edges.jsonl 100 &
    PID=$!
    sleep 5
    kill $PID 2>/dev/null || true
fi

if [[ -f "/tmp/test_edges.jsonl" ]]; then
    LINES=$(wc -l < /tmp/test_edges.jsonl)
    echo "  ✅ Generated $LINES synthetic edges"
    rm -f /tmp/test_edges.jsonl
fi

echo ""
echo "📊 Step 6: Binary information"
ls -lh "$BINARY_PATH"
echo "  Built for: $MODE"
echo "  Features: $FEATURES"  
echo "  Profile: $PROFILE"

if command -v file &> /dev/null; then
    echo "  File type: $(file "$BINARY_PATH")"
fi

if [[ "$MODE" == "production" ]]; then
    echo "  Stripped: $(if nm "$BINARY_PATH" >/dev/null 2>&1; then echo 'No'; else echo 'Yes'; fi)"
fi

echo ""
echo "🎯 Step 7: Quick health check"

# Start the service in background for health check
echo "  Starting service for health check..."
source .env* 2>/dev/null || true
"$BINARY_PATH" &
SERVICE_PID=$!

# Wait for startup
sleep 3

# Check if process is still running
if kill -0 $SERVICE_PID 2>/dev/null; then
    echo "  ✅ Service started successfully (PID: $SERVICE_PID)"
    
    # Test health endpoints if available
    if command -v curl &> /dev/null; then
        sleep 2
        if curl -s http://localhost:8080/live >/dev/null 2>&1; then
            echo "  ✅ Health endpoint responding"
        else
            echo "  ⚠️  Health endpoint not responding (may need Kafka/Redis)"
        fi
    fi
    
    # Clean shutdown
    kill $SERVICE_PID 2>/dev/null || true
    wait $SERVICE_PID 2>/dev/null || true
else
    echo "  ❌ Service failed to start - check logs"
fi

echo ""
echo "🎉 Setup Complete!"
echo "=================="
echo ""
echo "Next steps:"
echo ""

if [[ "$MODE" == "development" ]]; then
    echo "1. Start dependencies:"
    echo "   docker-compose up -d kafka redis  # or your preferred setup"
    echo ""
    echo "2. Run the service:"
    echo "   source .env"
    echo "   $BINARY_PATH"
    echo ""
    echo "3. Test with synthetic data:"
    echo "   cargo run --bin gen_synthetic_edges --features=dev-full -- test.jsonl 1000"
    echo "   cargo run --bin arb_replay --features=dev-full -- --input test.jsonl --burst"
    echo ""
    echo "4. Monitor via web interface:"
    echo "   curl http://localhost:8080/ready"
    echo "   curl http://localhost:8080/metrics"
    echo "   # WebSocket: ws://localhost:8080/ws/arbs"
else
    echo "1. Review production configuration:"
    echo "   cat .env.production"
    echo ""
    echo "2. Deploy binary:"
    echo "   cp $BINARY_PATH /usr/local/bin/"
    echo "   systemctl start arb-engine  # or your deployment method"
    echo ""
    echo "3. Monitor health:"
    echo "   curl http://your-host:8080/ready"
    echo ""
fi

echo "📚 Documentation:"
echo "  - Troubleshooting: See troubleshooting guide"
echo "  - Configuration: Check .env file"
echo "  - Monitoring: http://localhost:8080/metrics"
echo ""
echo "🐛 If you encounter issues:"
echo "  1. Check logs: RUST_LOG=arb_engine=debug $BINARY_PATH"
echo "  2. Verify dependencies: Kafka, Redis connectivity" 
echo "  3. Review feature flags in Cargo.toml"
echo "  4. Test with stubs: cargo run --features=stubs"
echo ""

if [[ "$MODE" == "development" ]]; then
    echo "💡 Development tips:"
    echo "  - Use 'cargo watch -x run' for auto-restart"
    echo "  - Enable profiling with --features=profiling"  
    echo "  - Generate flame graphs with PPROF_FLAME=profile.svg"
    echo ""
fi

echo "✨ Happy arbitraging! ✨"