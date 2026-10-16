#!/bin/bash

# MEVana Geyser Stream Automatic Recovery System
# Monitors Geyser streams and automatically recovers failed ones
# Usage: ./geyser_recovery.sh [stream_id]

set -e

# Configuration
GEYSER_ENDPOINT="https://grpc.ny.shyft.to"
X_TOKEN="<SHYFT_X_TOKEN>"
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb"
METRICS_PORT=8088
RECOVERY_LOG_DIR="/opt/MEVana/logs/recovery"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }

# Create recovery log directory
mkdir -p "$RECOVERY_LOG_DIR"

# Function to send Telegram alert (placeholder for future implementation)
send_telegram_alert() {
    local message="$1"
    local severity="$2"

    # TODO: Implement Telegram bot integration
    log_info "TELEGRAM ALERT [$severity]: $message"

    # Log to file for now
    echo "$(date '+%Y-%m-%d %H:%M:%S') [$severity] $message" >> "$RECOVERY_LOG_DIR/telegram_alerts.log"
}

# Function to check stream health via metrics endpoint
check_stream_health() {
    local stream_id=$1
    local health_metric

    health_metric=$(curl -s "http://localhost:${METRICS_PORT}/metrics" | \
        grep "multi_geyser_stream_healthy{stream_id=\"$stream_id\"}" | \
        awk '{print $2}' 2>/dev/null || echo "unknown")

    echo "$health_metric"
}

# Function to check time since last message
check_stream_staleness() {
    local stream_id=$1
    local seconds_since_last

    seconds_since_last=$(curl -s "http://localhost:${METRICS_PORT}/metrics" | \
        grep "multi_geyser_stream_seconds_since_last_message{stream_id=\"$stream_id\"}" | \
        awk '{print $2}' 2>/dev/null || echo "unknown")

    echo "$seconds_since_last"
}

# Function to recover a specific Geyser stream
recover_geyser_stream() {
    local stream_id=$1
    log_info "🔄 Attempting to recover Geyser stream $stream_id..."

    # Kill existing processes for this stream
    log_info "Stopping existing processes for stream $stream_id..."
    pkill -f "hotpath_arb_engine.*stream.*$stream_id" || true
    pkill -f "METRICS_PORT=${METRICS_PORT}" || true
    sleep 5

    # Clear any port conflicts
    local pid_on_port=$(lsof -ti:${METRICS_PORT} 2>/dev/null || echo "")
    if [ -n "$pid_on_port" ]; then
        log_warning "Killing process $pid_on_port on port $METRICS_PORT"
        kill -9 "$pid_on_port" 2>/dev/null || true
        sleep 3
    fi

    # Start recovery with exponential backoff
    local recovery_log="$RECOVERY_LOG_DIR/recovery_stream_${stream_id}_$(date '+%Y%m%d_%H%M%S').log"

    log_info "Starting recovery process (log: $recovery_log)"

    # Use the clean startup approach with proper configuration
    nohup bash -c "
        export MULTI_ENDPOINT_MODE=true
        export PAPER_TRADING=true
        export DATABASE_URL='$DATABASE_URL'
        export GEYSER_ENDPOINT='$GEYSER_ENDPOINT'
        export X_TOKEN='$X_TOKEN'
        export METRICS_PORT=$METRICS_PORT
        export RUST_LOG=info

        cd /opt/MEVana || cd /Users/michaelinman/Repos/MEVana
        ./target/release/hotpath_arb_engine 2>&1
    " > "$recovery_log" 2>&1 &

    local recovery_pid=$!
    echo "$recovery_pid" > "$RECOVERY_LOG_DIR/stream_${stream_id}.pid"

    log_success "✅ Recovery process started for stream $stream_id (PID: $recovery_pid)"
    send_telegram_alert "Stream $stream_id recovery initiated (PID: $recovery_pid)" "medium"

    # Wait for startup and validate
    log_info "⏳ Waiting for stream startup..."
    sleep 30

    # Validate recovery success
    local retries=5
    for i in $(seq 1 $retries); do
        local new_health=$(check_stream_health "$stream_id")
        if [ "$new_health" = "1" ]; then
            log_success "✅ Stream $stream_id recovery successful! Health: $new_health"
            send_telegram_alert "Stream $stream_id recovered successfully" "medium"
            return 0
        else
            log_warning "⏳ Recovery validation attempt $i/$retries (health: $new_health)"
            sleep 10
        fi
    done

    log_error "❌ Stream $stream_id recovery failed after $retries attempts"
    send_telegram_alert "CRITICAL: Stream $stream_id recovery failed after multiple attempts" "critical"
    return 1
}

# Function to check all streams and recover failed ones
check_and_recover_all_streams() {
    log_info "🔍 Checking health of all Geyser streams..."

    local failed_streams=()
    local stalled_streams=()

    # Check streams 0, 1, 2 (typical multi-stream setup)
    for stream_id in 0 1 2; do
        local health=$(check_stream_health "$stream_id")
        local staleness=$(check_stream_staleness "$stream_id")

        log_info "Stream $stream_id: Health=$health, Staleness=${staleness}s"

        # Check for complete failure
        if [ "$health" = "0" ]; then
            log_error "❌ Stream $stream_id is DOWN (health check failed)"
            failed_streams+=("$stream_id")
        # Check for stalled streams (>120 seconds without message)
        elif [ "$staleness" != "unknown" ] && [ "$staleness" -gt 120 ]; then
            log_warning "⚠️ Stream $stream_id is STALLED (${staleness}s since last message)"
            stalled_streams+=("$stream_id")
        else
            log_success "✅ Stream $stream_id is healthy"
        fi
    done

    # Recover failed streams first
    for stream_id in "${failed_streams[@]}"; do
        log_error "🚨 CRITICAL: Recovering failed stream $stream_id"
        send_telegram_alert "CRITICAL: Geyser stream $stream_id is DOWN - initiating recovery" "critical"
        recover_geyser_stream "$stream_id"
    done

    # Recover stalled streams
    for stream_id in "${stalled_streams[@]}"; do
        log_warning "⚠️ RECOVERY: Recovering stalled stream $stream_id"
        send_telegram_alert "WARNING: Geyser stream $stream_id stalled - initiating recovery" "high"
        recover_geyser_stream "$stream_id"
    done

    if [ ${#failed_streams[@]} -eq 0 ] && [ ${#stalled_streams[@]} -eq 0 ]; then
        log_success "🎉 All Geyser streams are healthy!"
    fi
}

# Main execution
main() {
    log_info "🚀 MEVana Geyser Stream Recovery System"
    log_info "Recovery logs: $RECOVERY_LOG_DIR"

    if [ $# -eq 1 ]; then
        # Recover specific stream
        local stream_id="$1"
        log_info "Recovering specific stream: $stream_id"
        recover_geyser_stream "$stream_id"
    else
        # Check and recover all streams
        check_and_recover_all_streams
    fi

    log_success "🎯 Recovery operation completed"
}

# Run with error handling
trap 'log_error "Recovery failed on line $LINENO"' ERR
main "$@"