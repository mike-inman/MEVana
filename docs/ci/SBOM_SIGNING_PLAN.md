# SBOM & Image Signing Plan
Last updated: 2025-09-15

Goals:
- Generate a Software Bill of Materials (SBOM) for each container image (executor-service now; others later) using `syft` or `cyclonedx-cargo`.
- Sign container images with `cosign` using keyless signing (GitHub OIDC) to avoid managing static secrets.
- Attach SBOM as build artifact and optionally push as OCI artifact (`attest` with cosign once stable).

## Approach
1. Add step in docker build job:
   - Install syft (`anchore/syft-action`) to generate `sbom.spdx.json` or `sbom.cyclonedx.json` from built image.
   - Upload as artifact; optionally attach to GH Release or push to registry: `ghcr.io/<owner>/<image>:sha-<SHA>-sbom`.
2. Add cosign install (`sigstore/cosign-installer`) + keyless sign:
   ```bash
   cosign sign --yes ghcr.io/<owner>/<image>:sha-<SHA>
   cosign attest --predicate sbom.cyclonedx.json --type cyclonedx ghcr.io/<owner>/<image>:sha-<SHA>
   ```
3. Verification docs for consumers: `cosign verify --certificate-identity https://github.com/<owner>/<repo>/...`.
4. (Optional) Policy enforcement with `cosign verify` in deployment pipeline (staging & prod) before rollout.

## Open Tasks
- [ ] Integrate syft action post-build.
- [ ] Add cosign installer action; enable keyless signing.
- [ ] Store SBOM as artifact & attach attestation.
- [ ] Add verification step example in README.
- [ ] Add admission controller policy (future, if using Kubernetes + Kyverno / OPA Gatekeeper).

## Rationale
Early SBOM + signing reduces supply chain risk and prepares for downstream compliance (SLSA, SOC2). Keyless avoids secret rotation overhead.
