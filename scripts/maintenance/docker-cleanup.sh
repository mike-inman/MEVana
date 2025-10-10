#!/bin/bash
# Docker Volume Cleanup & Disk Monitoring
# Run this as a cron job: 0 2 * * * /opt/MEVana/scripts/maintenance/docker-cleanup.sh

set -euo pipefail

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
LOG_FILE="/opt/MEVana/logs/docker-cleanup.log"
DISK_THRESHOLD=80  # Alert if disk usage > 80%

# Logging function
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

log "=== Docker Cleanup Started ==="

# 1. Check disk usage
DISK_USAGE=$(df / | tail -1 | awk '{print $5}' | sed 's/%//')
log "Current disk usage: ${DISK_USAGE}%"

if [ "$DISK_USAGE" -gt "$DISK_THRESHOLD" ]; then
    log "WARNING: Disk usage above ${DISK_THRESHOLD}%!"
fi

# 2. Clean up orphaned Docker volumes
log "Cleaning orphaned Docker volumes..."
ORPHANED_COUNT=$(docker volume ls -qf dangling=true | wc -l)
log "Found $ORPHANED_COUNT orphaned volumes"

if [ "$ORPHANED_COUNT" -gt 0 ]; then
    BEFORE_SIZE=$(du -sh /var/lib/docker/volumes/ 2>/dev/null | awk '{print $1}')
    docker volume prune -f >> "$LOG_FILE" 2>&1
    AFTER_SIZE=$(du -sh /var/lib/docker/volumes/ 2>/dev/null | awk '{print $1}')
    log "Volume cleanup complete. Before: $BEFORE_SIZE, After: $AFTER_SIZE"
else
    log "No orphaned volumes to clean"
fi

# 3. Clean old Docker images
log "Cleaning old Docker images..."
docker image prune -a -f --filter "until=168h" >> "$LOG_FILE" 2>&1
log "Image cleanup complete"

# 4. Clean build cache
log "Cleaning Docker build cache..."
docker builder prune -f --filter "until=168h" >> "$LOG_FILE" 2>&1
log "Build cache cleanup complete"

# 5. Truncate large log files
log "Truncating container logs > 100MB..."
find /var/lib/docker/containers/ -name "*-json.log" -type f -size +100M -exec truncate -s 50M {} \; 2>/dev/null || true
log "Log truncation complete"

# 6. Clean MEVana application logs older than 7 days
log "Cleaning old MEVana logs..."
find /opt/MEVana/logs/ -name "*.log" -type f -mtime +7 -exec rm -f {} \; 2>/dev/null || true
log "MEVana log cleanup complete"

# 7. Final disk usage check
FINAL_USAGE=$(df / | tail -1 | awk '{print $5}' | sed 's/%//')
SAVED=$((DISK_USAGE - FINAL_USAGE))
log "Final disk usage: ${FINAL_USAGE}% (freed ${SAVED}%)"

log "=== Docker Cleanup Complete ==="

# Alert if still above threshold
if [ "$FINAL_USAGE" -gt "$DISK_THRESHOLD" ]; then
    log "CRITICAL: Disk usage still above ${DISK_THRESHOLD}% after cleanup!"
    # Optional: Send alert (Slack, email, etc.)
    # curl -X POST "$SLACK_WEBHOOK" -d "{\"text\":\"Disk usage critical: ${FINAL_USAGE}%\"}"
fi

exit 0
