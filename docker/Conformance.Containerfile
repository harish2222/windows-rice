# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: WINDOW-RICE Podman Linux Conformance Image
# ============================================================================
# Companion to docker/Dockerfile (Windows testenv): this image runs the
# cross-platform conformance subset under PODMAN (Linux containers) per the
# container-only verification policy (TEST_INFRA.md section 6). Windows-bound
# suites (MSVC cargo units + junction-based E2E tiers) remain engine-bound to
# Windows containers and execute on windows runners via ci.yml's e2e-suite job.
#
# Podman affordance: podman honors .dockerignore as the build-context filter
# when no .containerignore exists, so the same heavy dirs are kept out.
#
# Build:  podman build -f docker/Conformance.Containerfile -t window-rice/conformance .
# Run:    podman run --rm -e IMAGE_ID=$(podman image inspect window-rice/conformance --format '{{.Id}}') window-rice/conformance
# ============================================================================

FROM mcr.microsoft.com/powershell:ubuntu-22.04

# apt layer: git (repo integrity + ls-files audits), curl for rustup
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends git curl ca-certificates; \
    rm -rf /var/lib/apt/lists/*

# rustup minimal + rustfmt (format conformance without any cross-build attempt)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --component rustfmt --no-modify-path
ENV PATH="/root/.cargo/bin:${PATH}"

# PowerShell-YAML for spec/workflow conformance parsing
RUN pwsh -NoProfile -Command "Set-PSRepository PSGallery -InstallationPolicy Trusted; Install-Module PowerShell-Yaml -Scope AllUsers -Force" \
    && pwsh -NoProfile -Command "Import-Module PowerShell-Yaml; Write-Host 'PowerShell-YAML ready'"

# Conformance runner + repo payload (host working tree is never mounted)
WORKDIR /repo
COPY . .

LABEL org.opencontainers.image.title="window-rice-conformance" \
      org.opencontainers.image.description="WINDOW-RICE cross-platform conformance runner (podman/Linux) - HKDEVS / Window-Rice" \
      org.opencontainers.image.authors="HKDEVS / Project -: Window-Rice"

CMD ["pwsh", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "/repo/docker/run-conformance-in-container.ps1"]
