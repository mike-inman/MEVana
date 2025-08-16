# CI/CD (GitHub Actions)

## CI
- Build Rust workspace (`cargo build --workspace`)
- Run tests, codegen fixture tests
- Build Docker images per crate/service; tag with `branch-sha`
- Push to GHCR/ECR/GCR

## CD
- Staging: docker compose/k8s deploy from tagged images; smoke test
- Prod: promote staging tag; Blue/Green or rolling update; health checks

## Secrets
- Use OIDC to cloud or sealed-secrets; no secrets in repo
