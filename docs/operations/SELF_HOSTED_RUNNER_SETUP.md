Self-Hosted GitHub Actions Runner — Server Setup

Last updated: 2025-09-15

Prereqs
- Docker + docker compose installed and working
- Repo: https://github.com/mike-inman/MEVana

1) Create Docker network (once)
```bash
docker network create mevana_net 2>/dev/null || true
```

2) Install GitHub Actions runner (as a service)
```bash
# As root on the server
mkdir -p /opt/actions-runner && cd /opt/actions-runner
# Get a registration token from GitHub → Repo → Settings → Actions → Runners → New self-hosted runner
# Then run:
RUNNER_VERSION=2.317.0
curl -o actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz -L https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
tar xzf actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
# Configure (paste token and repo URL when prompted)
./config.sh --url https://github.com/mike-inman/MEVana --token <REG_TOKEN>
# Install as a service and start
sudo ./svc.sh install
sudo ./svc.sh start
```

3) Optional: add GHCR Secrets in GitHub (already present)
- GHCR_USERNAME, GHCR_TOKEN → lets the deploy job docker login ghcr.io for pulls when build_locally=false
- HOTPATH_ENDPOINT, HOTPATH_X_TOKEN → deploy job writes infrastructure/.env automatically

4) Deploy from GitHub (server builds)
- Actions → Self-Hosted Deploy → Run workflow
  - services: `hotpath` (or `hotpath normalizer discovery executor`)
  - build_locally: `true`
  - mevana_tag: leave empty

5) Diagnostics / Restart (on the runner)
- Actions → Self-Hosted Diagnostics → Run workflow (services input optional)
- Actions → Self-Hosted Restart → Run workflow (services input optional)

Notes
- This setup keeps builds on the server (fast, predictable). Switch build_locally=false later to pull from GHCR.
- The workflows use docker compose overlays: infrastructure/docker-compose.server.yml + infrastructure/docker-compose.full.yml

