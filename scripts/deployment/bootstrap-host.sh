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
ENV_DIR=${ENV_DIR:-/etc/mevana}
SERVICE_DIR=/etc/systemd/system

if [[ $EUID -ne 0 ]]; then
  echo "Run bootstrap-host.sh as root (or with sudo)" >&2
  exit 1
fi

# Ensure system group/user exist
if ! getent group "$MEVANA_GROUP" >/dev/null; then
  groupadd --system "$MEVANA_GROUP"
fi

if ! id -u "$MEVANA_USER" >/dev/null 2>&1; then
  useradd --system --no-create-home --home "$INSTALL_PREFIX" --shell /usr/sbin/nologin \
    --gid "$MEVANA_GROUP" "$MEVANA_USER"
fi

# Create application directories with safe permissions
install -d -m 0750 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$INSTALL_PREFIX"
install -d -m 0750 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$INSTALL_PREFIX/bin"
install -d -m 0750 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$INSTALL_PREFIX/config"
install -d -m 0750 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$INSTALL_PREFIX/logs"
install -d -m 0750 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$INSTALL_PREFIX/state"
install -d -m 0700 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$INSTALL_PREFIX/keys"
install -d -m 0750 -o "$MEVANA_USER" -g "$MEVANA_GROUP" "$INSTALL_PREFIX/releases"

# Seed the wallet key if present in the repository (first-time convenience only)
if [[ -f "$REPO_ROOT/infrastructure/keys/id.json" && ! -f "$INSTALL_PREFIX/keys/id.json" ]]; then
  install -m 0600 -o "$MEVANA_USER" -g "$MEVANA_GROUP" \
    "$REPO_ROOT/infrastructure/keys/id.json" "$INSTALL_PREFIX/keys/id.json"
fi

# Provide reference configuration files without clobbering operator edits
install -m 0640 -o "$MEVANA_USER" -g "$MEVANA_GROUP" \
  "$REPO_ROOT/config/flash_loan_providers.mainnet.template.json" \
  "$INSTALL_PREFIX/config/flash_loan_providers.mainnet.template.json"
if [[ ! -f "$INSTALL_PREFIX/config/flash_loan_providers.mainnet.json" ]]; then
  cp "$INSTALL_PREFIX/config/flash_loan_providers.mainnet.template.json" \
    "$INSTALL_PREFIX/config/flash_loan_providers.mainnet.json"
  chown "$MEVANA_USER:$MEVANA_GROUP" "$INSTALL_PREFIX/config/flash_loan_providers.mainnet.json"
  chmod 0640 "$INSTALL_PREFIX/config/flash_loan_providers.mainnet.json"
fi
install -m 0640 -o "$MEVANA_USER" -g "$MEVANA_GROUP" \
  "$REPO_ROOT/config/flash_loan_providers.local.json" \
  "$INSTALL_PREFIX/config/flash_loan_providers.local.json"

# Ensure environment directory exists (store canonical env + per-service overrides)
install -d -m 0750 "$ENV_DIR"

# Drop sample environment files for operators to fill in
install -m 0640 -o root -g "$MEVANA_GROUP" "$REPO_ROOT/config/environments/common.env.sample" \
  "$ENV_DIR/common.env.sample"
install -m 0640 -o root -g "$MEVANA_GROUP" "$REPO_ROOT/config/environments/hotpath.env.sample" \
  "$ENV_DIR/hotpath.env.sample"
install -m 0640 -o root -g "$MEVANA_GROUP" "$REPO_ROOT/config/environments/normalizer.env.sample" \
  "$ENV_DIR/normalizer.env.sample"
install -m 0640 -o root -g "$MEVANA_GROUP" "$REPO_ROOT/config/environments/discovery.env.sample" \
  "$ENV_DIR/discovery.env.sample"
install -m 0640 -o root -g "$MEVANA_GROUP" "$REPO_ROOT/config/environments/executor.env.sample" \
  "$ENV_DIR/executor.env.sample"

# Install/refresh systemd unit files
install -m 0644 "$REPO_ROOT/infrastructure/systemd/hotpath-arb-engine.service" \
  "$SERVICE_DIR/hotpath-arb-engine.service"
install -m 0644 "$REPO_ROOT/infrastructure/systemd/normalizer-service.service" \
  "$SERVICE_DIR/normalizer-service.service"
install -m 0644 "$REPO_ROOT/infrastructure/systemd/discovery-service.service" \
  "$SERVICE_DIR/discovery-service.service"
install -m 0644 "$REPO_ROOT/infrastructure/systemd/executor-service.service" \
  "$SERVICE_DIR/executor-service.service"

systemctl daemon-reload

echo "Bootstrap complete. Populate $ENV_DIR/common.env and per-service env files, then enable services." >&2
