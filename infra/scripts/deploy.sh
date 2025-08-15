#!/bin/bash

# Solana AI Trading Platform Deployment Script
# Usage: ./deploy.sh [environment] [version]
# Example: ./deploy.sh production v1.0.0

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
ENVIRONMENT=${1:-development}
VERSION=${2:-latest}
REGISTRY=${REGISTRY:-ghcr.io}
REPOSITORY=${REPOSITORY:-your-username/solana-ai-trading}

# Environment-specific configurations
case $ENVIRONMENT in
    development)
        COMPOSE_FILE="infrastructure/docker-compose.yml"
        HEALTH_CHECK_URL="http://localhost:8080"
        EXPECTED_TOPICS=6
        ;;
    staging)
        COMPOSE_FILE="infrastructure/docker-compose.staging.yml"
        HEALTH_CHECK_URL="https://staging-kafka.yourdomain.com"
        EXPECTED_TOPICS=6
        ;;
    production)
        COMPOSE_FILE="infrastructure/docker-compose.production.yml"
        HEALTH_CHECK_URL="https://kafka.yourdomain.com"
        EXPECTED_TOPICS=10
        ;;
    *)
        echo -e "${RED}Error: Invalid environment. Use: development, staging, or production${NC}"
        exit 1
        ;;
esac

echo -e "${BLUE}🚀 Deploying Solana AI Trading Platform${NC}"
echo -e "${BLUE}Environment: ${YELLOW}$ENVIRONMENT${NC}"
echo -e "${BLUE}Version: ${YELLOW}$VERSION${NC}"
echo ""

# Function to print banner
print_banner() {
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║               🤖 AI Trading Platform Deployment              ║"
    echo "║                                                              ║"
    echo "║  📊 Real-time Swap Detection    🧠 AI Agent Coordination     ║"
    echo "║  ⚡ Kafka Streaming            🎯 Arbitrage Detection        ║"
    echo "║  🔴 Redis Cache                💰 Trading Strategies         ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Function to check prerequisites
check_prerequisites() {
    echo -e "${BLUE}🔍 Checking prerequisites...${NC}"
    
    # Check Docker
    if ! docker info > /dev/null 2>&1; then
        echo -e "${RED}❌ Docker is not running${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Docker is running${NC}"
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        echo -e "${RED}❌ Docker Compose is not installed${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Docker Compose is available${NC}"
    
    # Check environment file
    if [ ! -f ".env" ]; then
        echo -e "${YELLOW}⚠️  .env file not found, creating template...${NC}"
        create_env_template
    fi
    echo -e "${GREEN}✅ Environment configuration exists${NC}"
    
    # Check Rust toolchain (for development)
    if [ "$ENVIRONMENT" = "development" ]; then
        if ! command -v cargo &> /dev/null; then
            echo -e "${YELLOW}⚠️  Cargo not found - needed for development builds${NC}"
        else
            echo -e "${GREEN}✅ Rust toolchain available${NC}"
        fi
    fi
}

# Function to create environment template
create_env_template() {
    cat > .env << 'EOF'
# Shyft Configuration
SHYFT_GRPC_ENDPOINT=https://grpc.us1.shyft.to
SHYFT_GRPC_TOKEN=your-token-here

# Infrastructure
REDIS_URL=redis://127.0.0.1:6379
KAFKA_BROKERS=localhost:29092

# Trading Configuration
ENABLE_ENRICHMENT=true
BATCH_SIZE=50
FLUSH_INTERVAL_MS=100

# AI Agent Configuration (for future use)
ENABLE_AI_AGENTS=false
AGENT_VOTING_THRESHOLD=0.7
RISK_LIMIT_PERCENT=5.0

# Monitoring
LOG_LEVEL=info
METRICS_ENABLED=true
EOF
    echo -e "${YELLOW}📝 Created .env template - please update with your credentials${NC}"
}

# Function to build Rust services
build_rust_services() {
    if [ "$ENVIRONMENT" = "development" ]; then
        echo -e "${BLUE}🔨 Building Rust services...${NC}"
        
        # Build workspace
        echo -e "${YELLOW}Building shared types...${NC}"
        cargo build --package solana-swap-types --release
        
        echo -e "${YELLOW}Building Shyft gRPC client...${NC}"
        cargo build --package shyft-grpc-client --release
        
        echo -e "${YELLOW}Building Kafka producer...${NC}"
        cargo build --package kafka-producer --release
        
        echo -e "${GREEN}✅ Rust services built successfully${NC}"
    fi
}

# Function to deploy infrastructure
deploy_infrastructure() {
    echo -e "${BLUE}🏗️ Deploying infrastructure...${NC}"
    
    # Stop existing services gracefully
    echo -e "${YELLOW}Stopping existing services...${NC}"
    docker-compose -f $COMPOSE_FILE down --remove-orphans --timeout 30
    
    # Clean up old containers (optional)
    if [ "$ENVIRONMENT" = "development" ]; then
        docker system prune -f --volumes || echo -e "${YELLOW}⚠️  Docker prune failed${NC}"
    fi
    
    # Start infrastructure services
    echo -e "${YELLOW}Starting infrastructure services...${NC}"
    docker-compose -f $COMPOSE_FILE up -d redis zookeeper kafka kafka-ui
    
    # Wait for services to be ready
    echo -e "${YELLOW}Waiting for infrastructure to be ready...${NC}"
    sleep 45
}

# Function to create Kafka topics
create_topics() {
    echo -e "${BLUE}📋 Creating Kafka topics...${NC}"
    
    # Run our topic creation script
    if [ -f "infrastructure/create-topics.sh" ]; then
        chmod +x infrastructure/create-topics.sh
        ./infrastructure/create-topics.sh
    else
        echo -e "${YELLOW}⚠️  Topic creation script not found, creating topics manually...${NC}"
        create_topics_manual
    fi
}

# Function to create topics manually
create_topics_manual() {
    local kafka_container=$(docker ps --format "table {{.Names}}" | grep kafka | grep -v ui)
    
    if [ -n "$kafka_container" ]; then
        echo -e "${YELLOW}Creating trading topics...${NC}"
        
        local topics=(
            "raw.swaps.raydium:3"
            "raw.swaps.orca:3"
            "raw.swaps.pumpfun:3"
            "enriched.swaps.all:6"
            "enriched.swaps.large:3"
            "alerts.arbitrage.opportunities:3"
        )
        
        for topic_config in "${topics[@]}"; do
            IFS=':' read -r topic partitions <<< "$topic_config"
            docker exec $kafka_container kafka-topics \
                --create --if-not-exists \
                --topic $topic \
                --bootstrap-server localhost:9092 \
                --partitions $partitions \
                --replication-factor 1
        done
        
        echo -e "${GREEN}✅ Topics created successfully${NC}"
    else
        echo -e "${RED}❌ Kafka container not found${NC}"
        return 1
    fi
}

# Function to run comprehensive health checks
health_check() {
    echo -e "${BLUE}🏥 Running health checks...${NC}"
    
    local max_attempts=20
    local attempt=1
    local all_healthy=false
    
    while [ $attempt -le $max_attempts ] && [ "$all_healthy" = false ]; do
        echo -e "${YELLOW}Health check attempt $attempt/$max_attempts...${NC}"
        
        local checks_passed=0
        local total_checks=5
        
        # Check 1: Redis connectivity
        if redis-cli ping > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Redis: Healthy${NC}"
            checks_passed=$((checks_passed + 1))
        else
            echo -e "${RED}❌ Redis: Unhealthy${NC}"
        fi
        
        # Check 2: Kafka connectivity
        local kafka_container=$(docker ps --format "table {{.Names}}" | grep kafka | grep -v ui)
        if [ -n "$kafka_container" ] && docker exec $kafka_container kafka-topics --bootstrap-server localhost:9092 --list > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Kafka: Healthy${NC}"
            checks_passed=$((checks_passed + 1))
        else
            echo -e "${RED}❌ Kafka: Unhealthy${NC}"
        fi
        
        # Check 3: Kafka UI accessibility
        if curl -s "$HEALTH_CHECK_URL" > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Kafka UI: Accessible${NC}"
            checks_passed=$((checks_passed + 1))
        else
            echo -e "${RED}❌ Kafka UI: Not accessible${NC}"
        fi
        
        # Check 4: Expected topics exist
        local topic_count=$(docker exec $kafka_container kafka-topics --bootstrap-server localhost:9092 --list 2>/dev/null | wc -l)
        if [ "$topic_count" -ge "$EXPECTED_TOPICS" ]; then
            echo -e "${GREEN}✅ Topics: $topic_count topics found${NC}"
            checks_passed=$((checks_passed + 1))
        else
            echo -e "${RED}❌ Topics: Only $topic_count topics found (expected $EXPECTED_TOPICS)${NC}"
        fi
        
        # Check 5: Container health
        local unhealthy_containers=$(docker-compose -f $COMPOSE_FILE ps | grep -c "unhealthy\|Exit" || echo "0")
        if [ "$unhealthy_containers" -eq 0 ]; then
            echo -e "${GREEN}✅ Containers: All healthy${NC}"
            checks_passed=$((checks_passed + 1))
        else
            echo -e "${RED}❌ Containers: $unhealthy_containers unhealthy${NC}"
        fi
        
        # Check if all passed
        if [ $checks_passed -eq $total_checks ]; then
            all_healthy=true
            echo -e "${GREEN}🎉 All health checks passed!${NC}"
            return 0
        else
            echo -e "${YELLOW}⚠️  $checks_passed/$total_checks checks passed${NC}"
        fi
        
        sleep 15
        attempt=$((attempt + 1))
    done
    
    echo -e "${RED}❌ Health checks failed after $max_attempts attempts${NC}"
    return 1
}

# Function to run trading platform smoke tests
smoke_tests() {
    echo -e "${BLUE}🧪 Running trading platform smoke tests...${NC}"
    
    # Test 1: Data pipeline connectivity
    echo -e "${YELLOW}Testing data pipeline...${NC}"
    
    # Check Redis queue lengths
    local swap_events=$(redis-cli llen swap_events 2>/dev/null || echo "0")
    echo -e "${CYAN}📊 Redis swap_events queue: $swap_events${NC}"
    
    # Test 2: Kafka topic health
    echo -e "${YELLOW}Testing Kafka topics...${NC}"
    local kafka_container=$(docker ps --format "table {{.Names}}" | grep kafka | grep -v ui)
    
    if [ -n "$kafka_container" ]; then
        echo -e "${CYAN}📋 Available topics:${NC}"
        docker exec $kafka_container kafka-topics --bootstrap-server localhost:9092 --list | while read topic; do
            echo -e "${CYAN}  - $topic${NC}"
        done
    fi
    
    # Test 3: Service configuration
    echo -e "${YELLOW}Testing service configuration...${NC}"
    
    if [ -f ".env" ]; then
        if grep -q "your-token-here" .env; then
            echo -e "${YELLOW}⚠️  Shyft token not configured in .env${NC}"
        else
            echo -e "${GREEN}✅ Shyft token configured${NC}"
        fi
    fi
    
    # Test 4: Build test (development only)
    if [ "$ENVIRONMENT" = "development" ]; then
        echo -e "${YELLOW}Testing Rust builds...${NC}"
        if cargo check --workspace --quiet; then
            echo -e "${GREEN}✅ All Rust projects compile${NC}"
        else
            echo -e "${RED}❌ Rust compilation issues${NC}"
            return 1
        fi
    fi
    
    echo -e "${GREEN}✅ Smoke tests completed${NC}"
    return 0
}

# Function to show comprehensive status
show_status() {
    echo -e "${BLUE}📊 Trading Platform Status${NC}"
    echo ""
    
    # Container status
    echo -e "${PURPLE}🐳 Container Status:${NC}"
    docker-compose -f $COMPOSE_FILE ps
    echo ""
    
    # Service URLs
    echo -e "${PURPLE}🌐 Service URLs:${NC}"
    echo -e "${GREEN}Kafka UI:     ${YELLOW}http://localhost:8080${NC} (Monitor topics & messages)"
    echo -e "${GREEN}Redis:        ${YELLOW}localhost:6379${NC} (Data cache)"
    echo -e "${GREEN}Kafka:        ${YELLOW}localhost:29092${NC} (External access)"
    echo ""
    
    # Data pipeline status
    echo -e "${PURPLE}📈 Data Pipeline Status:${NC}"
    local swap_events=$(redis-cli llen swap_events 2>/dev/null || echo "0")
    local json_events=$(redis-cli llen swap_events_json 2>/dev/null || echo "0")
    echo -e "${CYAN}Redis Queues:${NC}"
    echo -e "  - swap_events: $swap_events"
    echo -e "  - swap_events_json: $json_events"
    
    # Kafka topics
    local kafka_container=$(docker ps --format "table {{.Names}}" | grep kafka | grep -v ui)
    if [ -n "$kafka_container" ]; then
        echo -e "${CYAN}Kafka Topics:${NC}"
        docker exec $kafka_container kafka-topics --bootstrap-server localhost:9092 --list 2>/dev/null | while read topic; do
            echo -e "  - $topic"
        done
    fi
    echo ""
    
    # Quick start commands
    echo -e "${PURPLE}🚀 Quick Start Commands:${NC}"
    echo -e "${YELLOW}Start Kafka Producer:${NC}   cd kafka-producer && cargo run"
    echo -e "${YELLOW}Start Shyft Client:${NC}     cd shyft-grpc-client && cargo run"
    echo -e "${YELLOW}Monitor Pipeline:${NC}       ./tools/monitor-kafka.sh"
    echo -e "${YELLOW}Health Check:${NC}           ./deploy.sh --status"
    echo ""
    
    # Recent logs
    echo -e "${PURPLE}📋 Recent Infrastructure Logs:${NC}"
    docker-compose -f $COMPOSE_FILE logs --tail=3 --timestamps
}

# Function to start trading services
start_trading_services() {
    if [ "$ENVIRONMENT" = "development" ]; then
        echo -e "${BLUE}🚀 Starting trading services...${NC}"
        echo -e "${YELLOW}Services should be started manually for development:${NC}"
        echo ""
        echo -e "${CYAN}Terminal 1 - Kafka Producer:${NC}"
        echo -e "cd kafka-producer && KAFKA_BROKERS=localhost:29092 cargo run"
        echo ""
        echo -e "${CYAN}Terminal 2 - Shyft Client:${NC}"
        echo -e "cd shyft-grpc-client && SHYFT_GRPC_TOKEN=your-token cargo run"
        echo ""
        echo -e "${CYAN}Terminal 3 - Monitor:${NC}"
        echo -e "./tools/monitor-kafka.sh"
    fi
}

# Function to rollback deployment
rollback() {
    echo -e "${RED}🔄 Rolling back deployment...${NC}"
    
    # Stop current services
    docker-compose -f $COMPOSE_FILE down --timeout 30
    
    # Could implement version management here
    echo -e "${YELLOW}💡 In production, this would restore previous version${NC}"
    echo -e "${GREEN}✅ Rollback completed${NC}"
}

# Function to cleanup
cleanup() {
    echo -e "${RED}🧹 Cleaning up deployment...${NC}"
    
    # Stop all services
    docker-compose -f $COMPOSE_FILE down --volumes --remove-orphans
    
    # Clean Docker system (careful in production!)
    if [ "$ENVIRONMENT" = "development" ]; then
        docker system prune -af --volumes
        echo -e "${GREEN}✅ Development cleanup completed${NC}"
    else
        echo -e "${YELLOW}⚠️  Production cleanup - only stopped services${NC}"
    fi
}

# Main deployment process
main() {
    print_banner
    
    echo -e "${BLUE}🚀 Starting deployment process...${NC}"
    
    # Pre-deployment
    check_prerequisites
    build_rust_services
    
    # Core deployment
    deploy_infrastructure
    create_topics
    
    # Post-deployment validation
    if health_check; then
        if smoke_tests; then
            show_status
            start_trading_services
            echo -e "${GREEN}🎉 Deployment successful!${NC}"
            echo -e "${CYAN}💡 Visit http://localhost:8080 to monitor your trading platform${NC}"
        else
            echo -e "${RED}❌ Smoke tests failed${NC}"
            rollback
            exit 1
        fi
    else
        echo -e "${RED}❌ Health checks failed${NC}"
        rollback
        exit 1
    fi
}

# Handle script arguments
case "${1:-}" in
    --help|-h)
        echo "Solana AI Trading Platform Deployment"
        echo ""
        echo "Usage: $0 [COMMAND] [ENVIRONMENT] [VERSION]"
        echo ""
        echo "Commands:"
        echo "  deploy      Deploy the platform (default)"
        echo "  --status    Show platform status"
        echo "  --rollback  Rollback deployment"
        echo "  --cleanup   Clean up all resources"
        echo ""
        echo "Environments: development, staging, production"
        echo "Example: $0 development v1.0.0"
        exit 0
        ;;
    --status)
        show_status
        exit 0
        ;;
    --rollback)
        rollback
        exit 0
        ;;
    --cleanup)
        cleanup
        exit 0
        ;;
esac

# Run main deployment
main "$@"