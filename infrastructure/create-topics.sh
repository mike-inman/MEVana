#!/bin/bash

echo "🚀 Creating Kafka topics for trading platform..."

# Find the Kafka container (should be infrastructure_kafka_1 or similar)
KAFKA_CONTAINER=$(docker ps --format "table {{.Names}}" | grep kafka | grep -v ui)

if [ -z "$KAFKA_CONTAINER" ]; then
    echo "❌ Kafka container not found!"
    echo "Available containers:"
    docker ps --format "table {{.Names}}\t{{.Status}}"
    exit 1
fi

echo "📋 Using Kafka container: $KAFKA_CONTAINER"

# Check if Kafka is ready
echo "🔍 Checking if Kafka is ready..."
docker exec $KAFKA_CONTAINER kafka-topics --bootstrap-server localhost:9092 --list > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "⚠️  Kafka not ready yet. Waiting 30 seconds..."
    sleep 30
    docker exec $KAFKA_CONTAINER kafka-topics --bootstrap-server localhost:9092 --list > /dev/null 2>&1
    if [ $? -ne 0 ]; then
        echo "❌ Kafka still not ready. Check container logs:"
        echo "docker logs $KAFKA_CONTAINER"
        exit 1
    fi
fi

echo "✅ Kafka is ready!"

# Function to create topic if it doesn't exist
create_topic() {
    local topic_name=$1
    local partitions=$2
    
    echo "Creating topic: $topic_name..."
    docker exec $KAFKA_CONTAINER kafka-topics \
        --create \
        --topic $topic_name \
        --bootstrap-server localhost:9092 \
        --partitions $partitions \
        --replication-factor 1 \
        --if-not-exists
    
    if [ $? -eq 0 ]; then
        echo "✅ Topic $topic_name created/verified"
    else
        echo "❌ Failed to create topic $topic_name"
    fi
}

# Create our trading topics
create_topic "raw.swaps.raydium" 3
create_topic "raw.swaps.orca" 3
create_topic "raw.swaps.pumpfun" 3
create_topic "enriched.swaps.all" 6
create_topic "enriched.swaps.large" 3
create_topic "alerts.arbitrage.opportunities" 3
create_topic "alerts.high.impact" 3

echo ""
echo "📋 All topics:"
docker exec $KAFKA_CONTAINER kafka-topics --list --bootstrap-server localhost:9092

echo ""
echo "🎉 Topic creation complete!"
echo "💡 Visit http://localhost:8080 to see Kafka UI"