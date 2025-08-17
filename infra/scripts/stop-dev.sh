# scripts/stop-dev.sh
#!/usr/bin/env bash
pkill -f 'cargo run -p dex-stream-orca' || true
pkill -f 'cargo run -p normalizer-orca' || true
pkill -f 'cargo run -p price-aggregator' || true
pkill -f 'cargo run -p ui-gateway' || true
pkill -f 'vite' || true
echo "Stopped."
