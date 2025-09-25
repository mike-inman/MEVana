#!/usr/bin/env bash
set -euo pipefail

if ! command -v git >/dev/null 2>&1; then
  echo "git is required to locate the repository root" >&2
  exit 1
fi

REPO_ROOT=$(git rev-parse --show-toplevel)
INSTALL_PREFIX=${INSTALL_PREFIX:-/opt/MEVana}
MEVANA_USER=${MEVANA_USER:-mevana}
MEVANA_GROUP=${MEVANA_GROUP:-$MEVANA_USER}
SYSTEMCTL=${SYSTEMCTL:-systemctl}
CARGO_CMD=${CARGO_CMD:-cargo}
SKIP_RESTART=${SKIP_RESTART:-0}
DEPLOY_TAG=${DEPLOY_TAG:-$(git rev-parse --short HEAD)}

USE_SYSTEMCTL=1
if [[ -z "$SYSTEMCTL" || "$SYSTEMCTL" == "false" ]]; then
  USE_SYSTEMCTL=0
elif ! command -v "$SYSTEMCTL" >/dev/null 2>&1; then
  USE_SYSTEMCTL=0
fi

if [[ $EUID -ne 0 ]]; then
  echo "Run release-services.sh as root (or with sudo)" >&2
  exit 1
fi

if [[ ! -d "$INSTALL_PREFIX/bin" ]]; then
  echo "Install prefix $INSTALL_PREFIX missing; run scripts/deployment/bootstrap-host.sh first" >&2
  exit 1
fi

map_service() {
  case "$1" in
    hotpath|hotpath-arb-engine|hotpath_arb_engine)
      echo "hotpath" ;;
    normalizer|normalizer-service)
      echo "normalizer" ;;
    discovery|discovery-service)
      echo "discovery" ;;
    executor|executor-service)
      echo "executor" ;;
    *)
      echo "unknown" ;;
  esac
}

# Default to all services when no arguments provided
if [[ $# -eq 0 ]]; then
  set -- hotpath normalizer discovery executor
fi

services_to_build=()
for raw in "$@"; do
  canonical=$(map_service "$raw")
  if [[ "$canonical" == "unknown" ]]; then
    echo "Skipping unknown service identifier '$raw'" >&2
    continue
  fi
  services_to_build+=("$canonical")
done

if [[ ${#services_to_build[@]} -eq 0 ]]; then
  echo "No valid services requested" >&2
  exit 1
fi

# Deduplicate services while preserving order
unique_services=()
for svc in "${services_to_build[@]}"; do
  duplicate=false
  for seen in "${unique_services[@]}"; do
    if [[ "$svc" == "$seen" ]]; then
      duplicate=true
      break
    fi
  done
  if [[ "$duplicate" == "false" ]]; then
    unique_services+=("$svc")
  fi
done

pushd "$REPO_ROOT" >/dev/null

units_to_restart=()
for svc in "${unique_services[@]}"; do
  case "$svc" in
    hotpath)
      crate="hotpath_arb_engine"
      bin="hotpath_arb_engine"
      unit="hotpath-arb-engine.service"
      ;;
    normalizer)
      crate="normalizer-service"
      bin="normalizer-service"
      unit="normalizer-service.service"
      ;;
    discovery)
      crate="discovery-service"
      bin="discovery-service"
      unit="discovery-service.service"
      ;;
    executor)
      crate="executor-service"
      bin="executor-service"
      unit="executor-service.service"
      ;;
  esac

  echo "Building $svc (crate=$crate, bin=$bin)"
  "$CARGO_CMD" build --release --package "$crate" --bin "$bin"

  src_path="$REPO_ROOT/target/release/$bin"
  dest_path="$INSTALL_PREFIX/bin/$bin"
  if [[ ! -f "$src_path" ]]; then
    echo "Expected binary $src_path not produced" >&2
    exit 1
  fi

  echo "Installing $bin to $dest_path"
  install -Dm755 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$src_path" "$dest_path"

  units_to_restart+=("$unit")
done

if [[ ${#units_to_restart[@]} -gt 0 && $USE_SYSTEMCTL -eq 1 ]]; then
  "$SYSTEMCTL" daemon-reload
  if [[ "$SKIP_RESTART" -ne 1 ]]; then
    for unit in "${units_to_restart[@]}"; do
      echo "Restarting $unit"
      "$SYSTEMCTL" enable "$unit" >/dev/null 2>&1 || true
      "$SYSTEMCTL" restart "$unit"
    done
    echo "Deployment complete"
    "$SYSTEMCTL" --no-pager status "${units_to_restart[@]}"
  else
    echo "SKIP_RESTART=1 set; skipping systemd restarts"
  fi
fi

echo "$DEPLOY_TAG" > "$INSTALL_PREFIX/.mevana_tag"
chown "$MEVANA_USER:$MEVANA_GROUP" "$INSTALL_PREFIX/.mevana_tag"

if [[ ${#units_to_restart[@]} -gt 0 && $USE_SYSTEMCTL -eq 0 ]]; then
  echo "Systemd disabled or unavailable; binaries installed without restarting services" >&2
fi

popd >/dev/null
