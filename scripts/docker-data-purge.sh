#!/bin/bash

# MEVana Docker Data Purge Script
# This script purges data from infrastructure containers while preserving schemas and settings
# Created: 2025-01-27

set -e

echo "🧹 Starting MEVana Docker data purge process..."
echo "📋 This will preserve schemas and settings but remove all data"
echo ""

# Function to check if container is running
check_container() {
    local container_name=$1
    if docker ps --format "table {{.Names}}" | grep -q "^${container_name}$"; then
        return 0
    else
        return 1
    fi
}

# Function to stop and remove containers
stop_and_remove_containers() {
    echo "🛑 Stopping and removing existing containers..."
    
    # Stop all running containers
    docker-compose -f infrastructure/docker-compose.yml down --remove-orphans
    
    # Remove any remaining containers
    docker container prune -f
    
    echo "✅ Containers stopped and removed"
    echo ""
}

# Function to purge volumes while preserving schemas
purge_volumes() {
    echo "🗑️  Purging data volumes..."
    
    # List current volumes
    echo "Current volumes:"
    docker volume ls --filter "name=mevana" --format "table {{.Name}}\t{{.Size}}"
    echo ""
    
    # Remove data volumes (this will remove all data but preserve volume definitions)
    echo "Removing data volumes..."
    docker volume rm $(docker volume ls --filter "name=mevana" --format "{{.Name}}") 2>/dev/null || true
    
    echo "✅ Data volumes purged"
    echo ""
}

# Function to clean up networks
cleanup_networks() {
    echo "🌐 Cleaning up networks..."
    
    # Remove unused networks
    docker network prune -f
    
    echo "✅ Networks cleaned up"
    echo ""
}

# Function to verify clean state
verify_clean_state() {
    echo "🔍 Verifying clean state..."
    
    echo "Remaining containers:"
    docker ps -a --format "table {{.Names}}\t{{.Status}}\t{{.Image}}"
    echo ""
    
    echo "Remaining volumes:"
    docker volume ls --format "table {{.Name}}\t{{.Size}}"
    echo ""
    
    echo "Remaining networks:"
    docker network ls --format "table {{.Name}}\t{{.Driver}}\t{{.Scope}}"
    echo ""
}

# Function to start fresh infrastructure
start_fresh_infrastructure() {
    echo "🚀 Starting fresh infrastructure..."
    
    # Start infrastructure services
    docker-compose -f infrastructure/docker-compose.yml up -d
    
    echo "✅ Infrastructure services started"
    echo ""
}

# Function to verify services are healthy
verify_services_health() {
    echo "🏥 Verifying service health..."
    
    # Wait for services to be ready
    echo "Waiting for services to be ready..."
    sleep 30
    
    # Check service status
    docker-compose -f infrastructure/docker-compose.yml ps
    
    echo ""
    echo "Service health status:"
    
    # Check PostgreSQL
    if check_container "mevana-postgres"; then
        echo "✅ PostgreSQL: Running"
    else
        echo "❌ PostgreSQL: Not running"
    fi
    
    # Check Zookeeper
    if check_container "mevana-zookeeper"; then
        echo "✅ Zookeeper: Running"
    else
        echo "❌ Zookeeper: Not running"
    fi
    
    # Check Kafka
    if check_container "mevana-kafka"; then
        echo "✅ Kafka: Running"
    else
        echo "❌ Kafka: Not running"
    fi
    
    # Check Redis
    if check_container "mevana-redis"; then
        echo "✅ Redis: Running"
    else
        echo "❌ Redis: Not running"
    fi
    
    # Check Prometheus
    if check_container "mevana-prometheus"; then
        echo "✅ Prometheus: Running"
    else
        echo "❌ Prometheus: Not running"
    fi
    
    # Check Grafana
    if check_container "mevana-grafana"; then
        echo "✅ Grafana: Running"
    else
        echo "❌ Grafana: Not running"
    fi
    
    echo ""
}

# Main execution
main() {
    echo "🎯 MEVana Docker Data Purge - Clean Fresh Start"
    echo "================================================"
    echo ""
    
    # Check if docker-compose.yml exists
    if [ ! -f "infrastructure/docker-compose.yml" ]; then
        echo "❌ Error: docker-compose.yml not found in infrastructure directory"
        exit 1
    fi
    
    # Check if Docker is running
    if ! docker info >/dev/null 2>&1; then
        echo "❌ Error: Docker is not running"
        exit 1
    fi
    
    echo "📁 Working directory: $(pwd)"
    echo "🐳 Docker Compose file: infrastructure/docker-compose.yml"
    echo ""
    
    # Execute purge steps
    stop_and_remove_containers
    purge_volumes
    cleanup_networks
    verify_clean_state
    
    echo "🔄 Ready to start fresh infrastructure..."
    echo ""
    
    # Ask user if they want to start services now
    read -p "Do you want to start the fresh infrastructure now? (y/N): " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        start_fresh_infrastructure
        verify_services_health
        
        echo "🎉 Fresh infrastructure setup complete!"
        echo ""
        echo "📊 Access points:"
        echo "   PostgreSQL: localhost:5432"
        echo "   Redis: localhost:6379"
        echo "   Kafka: localhost:9093"
        echo "   Prometheus: http://localhost:9090"
        echo "   Grafana: http://localhost:3000 (admin/admin)"
        echo ""
        echo "💡 Next steps:"
        echo "   1. Verify all services are healthy"
        echo "   2. Check logs for any errors: docker-compose logs"
        echo "   3. Begin adding your custom application containers"
        echo ""
    else
        echo "⏸️  Infrastructure ready but not started"
        echo "💡 To start later, run: docker-compose -f infrastructure/docker-compose.yml up -d"
        echo ""
    fi
}

# Run main function
main "$@"
