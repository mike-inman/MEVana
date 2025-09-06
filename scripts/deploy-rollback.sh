#!/bin/bash
# Deployment Time Machine - Because sometimes you just need to go back! ⏰
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

echo -e "${BLUE}"
cat << "EOF"
⏰ DEPLOYMENT TIME MACHINE ⏰
┌─────────────────────────────┐
│  "It was working yesterday" │
│     - Every developer       │
└─────────────────────────────┘
EOF
echo -e "${NC}"

SERVER_USER="${SERVER_USER:-root}"
SERVER_HOST="${SERVER_HOST:-149.28.56.125}"
DEPLOY_PATH="${DEPLOY_PATH:-/opt/MEVana}"

# Function to create deployment snapshot
create_snapshot() {
    local snapshot_name="$1"
    echo -e "${BLUE}📸 Creating snapshot: $snapshot_name${NC}"
    
    ssh -T $SERVER_USER@$SERVER_HOST << SNAPSHOT_EOF
        mkdir -p $DEPLOY_PATH/snapshots
        timestamp=\$(date +%Y%m%d_%H%M%S)
        snapshot_dir="$DEPLOY_PATH/snapshots/\${timestamp}_${snapshot_name}"
        
        echo "Creating snapshot at: \$snapshot_dir"
        mkdir -p "\$snapshot_dir"
        
        # Backup critical files
        cp -r $DEPLOY_PATH/target "\$snapshot_dir/" 2>/dev/null || true
        cp -r $DEPLOY_PATH/logs "\$snapshot_dir/" 2>/dev/null || true
        cp $DEPLOY_PATH/*.env "\$snapshot_dir/" 2>/dev/null || true
        
        # Save running processes
        pgrep -f "arb-engine|hotpath" > "\$snapshot_dir/running_processes.txt" 2>/dev/null || true
        
        # Save port usage
        lsof -i :8085,8088,29092 > "\$snapshot_dir/port_usage.txt" 2>/dev/null || true
        
        # Save git commit
        cd $DEPLOY_PATH
        git rev-parse HEAD > "\$snapshot_dir/git_commit.txt" 2>/dev/null || echo "unknown" > "\$snapshot_dir/git_commit.txt"
        
        echo "✅ Snapshot saved: \$snapshot_dir"
        echo "\$snapshot_dir" > $DEPLOY_PATH/.last_snapshot
SNAPSHOT_EOF
}

# Function to list snapshots
list_snapshots() {
    echo -e "${PURPLE}📚 Available snapshots:${NC}"
    ssh -T $SERVER_USER@$SERVER_HOST << LIST_EOF
        cd $DEPLOY_PATH/snapshots 2>/dev/null || { echo "No snapshots found"; exit 1; }
        for snapshot in \$(ls -1t); do
            if [[ -d "\$snapshot" ]]; then
                commit=\$(cat "\$snapshot/git_commit.txt" 2>/dev/null || echo "unknown")
                size=\$(du -sh "\$snapshot" | cut -f1)
                echo "  📸 \$snapshot (commit: \${commit:0:8}, size: \$size)"
            fi
        done
LIST_EOF
}

# Function to rollback to snapshot
rollback_to_snapshot() {
    local snapshot_name="$1"
    
    if [[ -z "$snapshot_name" ]]; then
        echo -e "${RED}❌ Please specify snapshot name${NC}"
        list_snapshots
        return 1
    fi
    
    echo -e "${YELLOW}⏰ Rolling back to: $snapshot_name${NC}"
    echo -e "${RED}⚠️  This will stop current services and restore previous state${NC}"
    read -p "Continue? (y/N): " -r
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Rollback cancelled"
        return 0
    fi
    
    ssh -T $SERVER_USER@$SERVER_HOST << ROLLBACK_EOF
        set -e
        snapshot_dir="$DEPLOY_PATH/snapshots/$snapshot_name"
        
        if [[ ! -d "\$snapshot_dir" ]]; then
            echo "❌ Snapshot not found: $snapshot_name"
            exit 1
        fi
        
        echo "🛑 Stopping current services..."
        pgrep -f "arb-engine|hotpath" | xargs kill 2>/dev/null || true
        sleep 3
        
        echo "📦 Restoring files..."
        cd $DEPLOY_PATH
        
        # Restore binaries
        if [[ -d "\$snapshot_dir/target" ]]; then
            rm -rf target
            cp -r "\$snapshot_dir/target" .
            echo "✅ Restored binaries"
        fi
        
        # Restore config
        if [[ -f "\$snapshot_dir/venue_config.env" ]]; then
            cp "\$snapshot_dir/venue_config.env" .
            echo "✅ Restored configuration"  
        fi
        
        # Restore git state
        if [[ -f "\$snapshot_dir/git_commit.txt" ]]; then
            commit=\$(cat "\$snapshot_dir/git_commit.txt")
            if [[ "\$commit" != "unknown" ]]; then
                git checkout "\$commit" 2>/dev/null || echo "⚠️  Could not restore git state"
            fi
        fi
        
        echo "🚀 Restarting services..."
        # Restart services with same ports/config as snapshot
        if [[ -f "\$snapshot_dir/running_processes.txt" ]] && [[ -s "\$snapshot_dir/running_processes.txt" ]]; then
            nohup ./target/release/arb-engine > logs/arb-engine.log 2>&1 &
            sleep 2
            nohup ./target/release/hotpath_arb_engine > logs/hotpath_arb_engine.log 2>&1 &
        fi
        
        echo "✅ Rollback complete!"
        echo "🔍 Check logs: tail -f logs/*.log"
ROLLBACK_EOF
}

# Function for emergency rollback (last working state)
emergency_rollback() {
    echo -e "${RED}🚨 EMERGENCY ROLLBACK INITIATED 🚨${NC}"
    
    # Find last successful snapshot
    local last_good=$(ssh -T $SERVER_USER@$SERVER_HOST << FIND_EOF
        cd $DEPLOY_PATH/snapshots 2>/dev/null || exit 1
        for snapshot in \$(ls -1t); do
            if [[ -d "\$snapshot" ]] && [[ "\$snapshot" == *"success"* ]]; then
                echo "\$snapshot"
                exit 0
            fi
        done
        # Fallback to most recent
        ls -1t | head -1
FIND_EOF
    )
    
    if [[ -n "$last_good" ]]; then
        echo -e "${GREEN}Found last good state: $last_good${NC}"
        rollback_to_snapshot "$last_good"
    else
        echo -e "${RED}❌ No snapshots available for emergency rollback${NC}"
        echo -e "${YELLOW}💡 Consider running a manual recovery${NC}"
    fi
}

# Function to cleanup old snapshots
cleanup_snapshots() {
    local keep_count="${1:-10}"
    echo -e "${BLUE}🧹 Cleaning up old snapshots (keeping $keep_count)${NC}"
    
    ssh -T $SERVER_USER@$SERVER_HOST << CLEANUP_EOF
        cd $DEPLOY_PATH/snapshots 2>/dev/null || exit 0
        snapshot_count=\$(ls -1 | wc -l)
        if [[ \$snapshot_count -gt $keep_count ]]; then
            echo "Found \$snapshot_count snapshots, removing oldest"
            ls -1t | tail -n +\$((keep_count + 1)) | xargs rm -rf
            echo "✅ Cleanup complete"
        else
            echo "Only \$snapshot_count snapshots, no cleanup needed"
        fi
CLEANUP_EOF
}

# Main command handling
case "${1:-help}" in
    "snapshot")
        create_snapshot "${2:-manual}"
        ;;
    "list")
        list_snapshots
        ;;
    "rollback")
        rollback_to_snapshot "$2"
        ;;
    "emergency")
        emergency_rollback
        ;;
    "cleanup")
        cleanup_snapshots "$2"
        ;;
    "help"|*)
        echo -e "${CYAN}Usage: $0 <command> [args]${NC}"
        echo ""
        echo "Commands:"
        echo "  snapshot [name]     - Create deployment snapshot"  
        echo "  list               - List available snapshots"
        echo "  rollback <name>    - Rollback to specific snapshot"
        echo "  emergency          - Emergency rollback to last good state"
        echo "  cleanup [keep]     - Remove old snapshots (default: keep 10)"
        echo ""
        echo "Examples:"
        echo "  $0 snapshot before_hotpath_changes"
        echo "  $0 rollback 20240106_143022_before_hotpath_changes"
        echo "  $0 emergency  # OH SH*T button"
        ;;
esac