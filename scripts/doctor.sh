#!/usr/bin/env bash
set -euo pipefail
echo "Repo: $(pwd)"
echo "Rust: $(rustc --version || echo missing)"
echo "Cargo: $(cargo --version || echo missing)"
echo "Docker: $(docker --version || echo missing)"
echo "Compose: $(docker compose version || echo missing)"
echo
echo "Docker check:"
docker ps >/dev/null && echo "OK: docker ps"
echo
echo "GitHub CLI:"
echo "$(gh --version 2>/dev/null || echo 'gh missing (optional)')"
echo
echo "SSH to prod alias:"
if ssh -o BatchMode=yes mevana-prod 'echo ok' 2>/dev/null; then
  echo "OK: ssh mevana-prod"
else
  echo "NOTE: configure ~/.ssh/config with Host mevana-prod"
fi