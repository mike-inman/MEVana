#!/usr/bin/env bash
set -euo pipefail

# Build Rust binaries locally (nightly), build runtime-only Docker images per service,
# and optionally ship to a remote server via docker load.

usage() {
  cat << USAGE
Usage: $0 --target (local|server) --services svc1,svc2 --tag TAG [--server user@host] [--remote-dir /opt/MEVana]

Services:
  hotpath, normalizer, discovery, executor

Examples:
  $0 --target local --services hotpath,normalizer --tag dev
  $0 --target server --services hotpath,normalizer,discovery,executor --tag $(git rev-parse --short HEAD) --server root@149.28.56.125
USAGE
}

TARGET=""
SERVICES=""
TAG="dev"
SERVER_HOST=""
REMOTE_DIR="/opt/MEVana"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target) TARGET="$2"; shift 2;;
    --services) SERVICES="$2"; shift 2;;
    --tag) TAG="$2"; shift 2;;
    --server) SERVER_HOST="$2"; shift 2;;
    --remote-dir) REMOTE_DIR="$2"; shift 2;;
    -h|--help) usage; exit 0;;
    *) echo "Unknown arg: $1"; usage; exit 1;;
  esac
done

if [[ -z "$TARGET" || -z "$SERVICES" ]]; then
  usage; exit 1
fi

IFS=',' read -r -a SVC_ARR <<< "$SERVICES"

echo "🔧 Ensuring nightly toolchain locally..."
if ! rustup toolchain list | grep -q nightly; then
  rustup toolchain install nightly
fi

build_bin() {
  local svc="$1"
  case "$svc" in
    hotpath)
      echo "🛠️  Building hotpath_arb_engine..."
      DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
        cargo +nightly build --release -p hotpath_arb_engine --bin hotpath_arb_engine
      ;;
    normalizer)
      echo "🛠️  Building normalizer-service (tap_orca_extract)..."
      cargo +nightly build --release -p normalizer-service --bin tap_orca_extract
      ;;
    discovery)
      echo "🛠️  Building discovery-service..."
      cargo +nightly build --release -p discovery-service
      ;;
    executor)
      echo "🛠️  Building executor-service..."
      cargo +nightly build --release -p executor-service
      ;;
    *) echo "Unknown service: $svc"; exit 1;;
  esac
}

build_image() {
  local svc="$1"; local image=""; local bin=""; local packages="ca-certificates curl"; local tmpdir=".build/${svc}"
  case "$svc" in
    hotpath)   image="mevana/hotpath-arb-engine:${TAG}"; bin="hotpath_arb_engine"; packages="ca-certificates librdkafka1 curl";;
    normalizer) image="mevana/normalizer-service:${TAG}"; bin="tap_orca_extract"; packages="ca-certificates librdkafka1 curl";;
    discovery) image="mevana/discovery-service:${TAG}"; bin="discovery-service"; packages="ca-certificates curl";;
    executor)  image="mevana/executor-service:${TAG}"; bin="executor-service"; packages="ca-certificates librdkafka1 curl";;
    *) echo "Unknown service for image: $svc"; exit 1;;
  esac
  echo "📦 Building image ${image}"
  rm -rf "$tmpdir" && mkdir -p "$tmpdir"
  # Create minimal Dockerfile in a clean context to avoid .dockerignore issues
  cat > "$tmpdir/Dockerfile" <<EOF
FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ${packages} \\
    && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY ${bin} /usr/local/bin/${bin}
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/${bin}"]
EOF
  cp "target/release/${bin}" "$tmpdir/${bin}"
  docker build -t "$image" "$tmpdir"
}

ship_image() {
  local svc="$1"; local image=""
  case "$svc" in
    hotpath) image="mevana/hotpath-arb-engine:${TAG}";;
    normalizer) image="mevana/normalizer-service:${TAG}";;
    discovery) image="mevana/discovery-service:${TAG}";;
    executor) image="mevana/executor-service:${TAG}";;
    *) echo "Unknown service for ship: $svc"; exit 1;;
  esac
  if [[ "$TARGET" == "server" ]]; then
    if [[ -z "$SERVER_HOST" ]]; then
      echo "--server is required when target=server"; exit 1
    fi
    echo "🚢 Shipping ${image} to ${SERVER_HOST}..."
    docker save "$image" | ssh "$SERVER_HOST" docker load
  fi
}

restart_remote() {
  local svc="$1"; local compose_svc=""
  case "$svc" in
    hotpath) compose_svc="hotpath-arb-engine";;
    normalizer) compose_svc="normalizer-service";;
    discovery) compose_svc="discovery-service";;
    executor) compose_svc="executor-service";;
  esac
  if [[ "$TARGET" == "server" ]]; then
    echo "🔄 Restarting ${compose_svc} on server..."
    ssh "$SERVER_HOST" "cd '${REMOTE_DIR}' && MEVANA_TAG='${TAG}' docker compose -p mevana -f infrastructure/docker-compose.server.yml -f infrastructure/docker-compose.full.yml up -d ${compose_svc}"
  fi
}

for s in "${SVC_ARR[@]}"; do
  build_bin "$s"
  build_image "$s"
  ship_image "$s"
  restart_remote "$s"
done

echo "✅ Done."
