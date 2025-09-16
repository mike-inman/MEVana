#!/usr/bin/env bash
set -euo pipefail

# Build all service images locally using their Dockerfiles (Linux multi-stage builds)
# Usage: scripts/development/build-images-locally.sh [tag]

TAG="${1:-$(git rev-parse --short=7 HEAD 2>/dev/null || date +%s)}"

build() {
  local name="$1" df="$2" img="ghcr.io/mike-inman/${name}:${TAG}"
  echo "\n🚧 Building ${name} with tag ${TAG} using ${df}"
  docker build -t "${img}" -f "${df}" .
}

# Core app services
build mevana-hotpath-arb-engine           crates/hotpath_arb_engine/Dockerfile
build mevana-normalizer-service           crates/normalizer-service/Dockerfile
build mevana-discovery-service            crates/discovery-service/Dockerfile
build mevana-executor-service             crates/executor-service/executor-service.Dockerfile

# Optional additional services (uncomment as needed)
# build mevana-edge-assembler               crates/edge-assembler/Dockerfile
# build mevana-arb-engine                   crates/arb-engine/Dockerfile
# build mevana-token-registry-service       crates/token-registry-service/Dockerfile
# build mevana-monitoring-service           crates/monitoring-service/Dockerfile

echo "\n✅ Done. Built images with tag: ${TAG}"
echo "   Set MEVANA_TAG=${TAG} when composing, e.g.:"
echo "   export MEVANA_TAG=${TAG} && docker compose -p mevana -f infrastructure/docker-compose.server.yml -f infrastructure/docker-compose.full.yml up -d hotpath-arb-engine"

