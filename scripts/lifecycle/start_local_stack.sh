#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_DIR="$REPO_ROOT/logs"
STATE_DIR="$REPO_ROOT/.local/mevana"
PID_DIR="$STATE_DIR/pids"
BIN_DIR="$REPO_ROOT/target/release"
COMPOSE_FILE="$REPO_ROOT/infrastructure/docker-compose.server.yml"
PROJECT_NAME="mevana"

mkdir -p "$LOG_DIR" "$PID_DIR"

info()  { printf '\033[0;34m[INFO]\033[0m %s\n' "$1"; }
success(){ printf '\033[0;32m[SUCCESS]\033[0m %s\n' "$1"; }
warn()  { printf '\033[1;33m[WARN]\033[0m %s\n' "$1"; }

info "Starting Docker infrastructure (project=$PROJECT_NAME)"
docker network create mevana_net >/dev/null 2>&1 || true
docker compose -p "$PROJECT_NAME" -f "$COMPOSE_FILE" up -d zookeeper kafka redis postgres adminer prometheus grafana kafka-ui

wait_for_container() {
  local name="$1"
  local timeout="${2:-180}"
  local waited=0
  while (( waited < timeout )); do
    local health
    health=$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "$name" 2>/dev/null || echo "")
    case "$health" in
      healthy|running)
        success "Container $name is $health"
        return 0
        ;;
    esac
    sleep 3
    waited=$((waited+3))
  done
  warn "Container $name did not become healthy in ${timeout}s"
  return 1
}

wait_for_container "${PROJECT_NAME}-zookeeper-1"
wait_for_container "${PROJECT_NAME}-kafka-1"
wait_for_container "${PROJECT_NAME}-redis-1"
wait_for_container "${PROJECT_NAME}-postgres-1"
wait_for_container "${PROJECT_NAME}-prometheus-1"
wait_for_container "${PROJECT_NAME}-grafana-1" 60 || true

info "Ensuring Kafka topics exist"
create_topic() {
  docker exec ${PROJECT_NAME}-kafka-1 kafka-topics --bootstrap-server localhost:9092 \
    --create --if-not-exists --topic "$1" --partitions 3 --replication-factor 1 >/dev/null
  success "Kafka topic '$1' ready"
}
create_topic dex-transactions
create_topic orca-swaps.parsed.v2
create_topic normalized-swaps.v1
create_topic unknown-programs
create_topic arb.opportunities.v1

info "Building MEVana binaries (this may take several minutes on first run)"
cargo build --release -p hotpath_arb_engine -p normalizer-service -p discovery-service -p executor-service >/dev/null
success "Release binaries built"

if [[ -f "$REPO_ROOT/.env" ]]; then
  info "Loading environment overrides from .env"
  set -a
  # shellcheck disable=SC1090
  source "$REPO_ROOT/.env"
  set +a
fi

DEFAULT_DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb"
DEFAULT_KAFKA="localhost:29092"
DEFAULT_REDIS="redis://127.0.0.1:6380/0"
DEFAULT_WALLET="$REPO_ROOT/infrastructure/keys/id.json"
DEFAULT_JITO="$REPO_ROOT/config/keys/jito-keypair.json"

DATABASE_URL="${DATABASE_URL:-$DEFAULT_DATABASE_URL}"
KAFKA_BROKER="${KAFKA_BROKER:-$DEFAULT_KAFKA}"
KAFKA_BOOTSTRAP_SERVERS="${KAFKA_BOOTSTRAP_SERVERS:-$DEFAULT_KAFKA}"
REDIS_URL="${REDIS_URL:-$DEFAULT_REDIS}"
WALLET_KEYPAIR_PATH="${WALLET_KEYPAIR_PATH:-$DEFAULT_WALLET}"
JITO_AUTH_KEYPAIR_PATH="${JITO_AUTH_KEYPAIR_PATH:-$DEFAULT_JITO}"

if [[ ! -f "$WALLET_KEYPAIR_PATH" ]]; then
  warn "Wallet keypair not found at $WALLET_KEYPAIR_PATH; falling back to $DEFAULT_WALLET"
  WALLET_KEYPAIR_PATH="$DEFAULT_WALLET"
fi

if [[ ! -f "$JITO_AUTH_KEYPAIR_PATH" ]]; then
  warn "Jito keypair not found at $JITO_AUTH_KEYPAIR_PATH; falling back to $DEFAULT_JITO"
  JITO_AUTH_KEYPAIR_PATH="$DEFAULT_JITO"
fi

mkdir -p "$PID_DIR"
for pidfile in "$PID_DIR"/*.pid; do
  [[ -e "$pidfile" ]] || continue
  if kill -0 "$(cat "$pidfile")" 2>/dev/null; then
    warn "Stopping existing $(basename "$pidfile" .pid) (pid $(cat "$pidfile"))"
    kill "$(cat "$pidfile")" 2>/dev/null || true
  fi
  rm -f "$pidfile"
done

start_service() {
  local name="$1"; shift
  local cmd=("$@")
  info "Starting $name"
  nohup "${cmd[@]}" >> "$LOG_DIR/local-$name.log" 2>&1 &
  local pid=$!
  echo "$pid" > "$PID_DIR/$name.pid"
  success "$name started (pid $pid, log $LOG_DIR/local-$name.log)"
}

start_service hotpath env \
  RUST_LOG="${RUST_LOG:-info}" \
  DATABASE_URL="$DATABASE_URL" \
  KAFKA_BROKER="$KAFKA_BROKER" \
  REDIS_URL="$REDIS_URL" \
  METRICS_PORT="${HOTPATH_METRICS_PORT:-8088}" \
  WALLET_KEYPAIR_PATH="$WALLET_KEYPAIR_PATH" \
  JITO_AUTH_KEYPAIR_PATH="$JITO_AUTH_KEYPAIR_PATH" \
  "$BIN_DIR/hotpath_arb_engine"

start_service normalizer env \
  RUST_LOG="${RUST_LOG:-info}" \
  DATABASE_URL="$DATABASE_URL" \
  KAFKA_BROKER="$KAFKA_BROKER" \
  REDIS_URL="$REDIS_URL" \
  METRICS_PORT="${NORMALIZER_METRICS_PORT:-9901}" \
  IN_TOPIC="${NORMALIZER_IN_TOPIC:-dex-transactions}" \
  OUT_TOPIC="${NORMALIZER_OUT_TOPIC:-orca-swaps.parsed.v2}" \
  GROUP_ID="${NORMALIZER_GROUP_ID:-normalizer-service}" \
  "$BIN_DIR/normalizer-service" pipe

start_service discovery env \
  RUST_LOG="${RUST_LOG:-info}" \
  KAFKA_BOOTSTRAP_SERVERS="$KAFKA_BOOTSTRAP_SERVERS" \
  REDIS_URL="$REDIS_URL" \
  DISCOVERY_METRICS_PORT="${DISCOVERY_METRICS_PORT:-9102}" \
  "$BIN_DIR/discovery-service"

start_service executor env \
  RUST_LOG="${RUST_LOG:-info}" \
  DATABASE_URL="$DATABASE_URL" \
  KAFKA_BROKER="$KAFKA_BROKER" \
  REDIS_URL="$REDIS_URL" \
  METRICS_PORT="${EXECUTOR_METRICS_PORT:-9904}" \
  HEALTH_CHECK_PORT="${EXECUTOR_HEALTH_PORT:-9898}" \
  WALLET_KEYPAIR_PATH="$WALLET_KEYPAIR_PATH" \
  ALLOW_EMPTY_SHYFT_API="${ALLOW_EMPTY_SHYFT_API:-1}" \
  RUN_PROVENANCE_ENABLED="${RUN_PROVENANCE_ENABLED:-0}" \
  "$BIN_DIR/executor-service"

success "Local MEVana stack started"
info "Check logs under $LOG_DIR/local-*.log and Grafana at http://localhost:3000"
