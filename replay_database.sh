#!/bin/bash

# Database Transaction Replay with Current Timestamps
# This updates your 174,008 recorded transactions to current time and loops them

echo "🎭 Database Transaction Replay - ISP Throttling Workaround"
echo ""
echo "This will:"
echo "  🔄 Update 174,008 recorded transactions to current timestamps"  
echo "  ♻️  Loop through your real transaction data continuously"
echo "  📊 Populate your Grafana dashboards with realistic historical patterns"
echo "  🚀 Bypass ISP bandwidth throttling completely"
echo ""

# Function to replay transactions in batches
replay_transactions() {
    local batch_size=100
    local delay_ms=200  # 200ms between batches = ~5 transactions/second
    local loop_count=0
    
    echo "🚀 Starting transaction replay loop..."
    
    while true; do
        loop_count=$((loop_count + 1))
        echo "🔄 Loop $loop_count: Replaying transactions with current timestamps..."
        
        # Update a batch of transactions to current timestamp
        docker exec postgres psql -U mev -d mevdb -c "
            UPDATE raw_dex_transactions 
            SET block_time = NOW() - (INTERVAL '1 second' * (id % 3600))
            WHERE id >= ($loop_count - 1) * $batch_size + 1 
            AND id <= $loop_count * $batch_size;
        " > /dev/null
        
        # Show progress
        echo "📥 Updated batch $loop_count ($batch_size transactions) - timestamps set to current time"
        
        # Simulate realistic transaction flow timing
        sleep 0.2
        
        # Reset to beginning when we've processed all transactions
        local max_id=$(docker exec postgres psql -U mev -d mevdb -t -c "SELECT MAX(id) FROM raw_dex_transactions;" | tr -d ' ')
        if [ $((loop_count * batch_size)) -gt $max_id ]; then
            echo "🔄 Completed full database cycle - restarting from beginning..."
            loop_count=0
            sleep 1
        fi
        
        # Show activity every 10 batches
        if [ $((loop_count % 10)) -eq 0 ]; then
            echo "📊 Progress: Processed $((loop_count * batch_size)) transactions (continuous loop active)"
        fi
    done
}

# Start the replay
replay_transactions