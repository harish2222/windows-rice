# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: WINDOW-RICE Windows Container Test Environment Image
# ============================================================================
# Policy: TEST_INFRA.md section 6 - every test suite (cargo units + the 4-tier
# E2E suite) executes ONLY inside this containerized test environment, never
# directly on the host desktop. Base = Windows Server 2022 core with pwsh 7,
# then MinGit + carapace + VS BuildTools (MSVC linker) + Rust stable.
#
# Build:  docker run-rice-tests.ps1   (builds with the repo as build context)
#   manual: docker build -f docker/Dockerfile -t window-rice/testenv .
# Run:    docker/run-rice-tests.ps1   (copies repo into image, scratch env)
# Provenance: every run stamps image id, OS build, and UTC timestamp into the
# output per the TEST_INFRA evidence policy.
# Note: Windows containers REQUIRE the Docker Windows engine (Docker Desktop
# switched to Windows containers). Linux engines (incl. podman-on-Windows,
# which only runs Linux containers) cannot execute MSVC + junction-based tests.
#
# Layer contract (TEST_INFRA.6 + repo-root guard):
#   C:\repo    - the repository working tree is COPIED into the image; the host
#                working directory is NEVER mounted (no stray writes possible).
#   C:\scratch - build/backup scratch root; the live master config is untouched.
#   .dockerignore keeps .git IN the context (tier tests run `git ls-files`).
# ============================================================================

FROM mcr.microsoft.com/powershell:windowsservercore-ltsc2022

SHELL ["pwsh", "-NoProfile", "-Command"]

# --- C:\tools + C:\scratch layout --------------------------------------------
RUN New-Item -ItemType Directory -Path C:\tools, C:\scratch, C:\scratch\.rice-backups -Force | Out-Null

# --- MinGit (portable, no shell integration) ---------------------------------
# Resolved dynamically from the git-for-windows latest release asset list so a
# build never depends on a hardcoded version matching an old asset name.
# Falls back to a known-good MinGit release when the GitHub API is rate-limited.
RUN $rel = $null; \
    try { $rel = Invoke-RestMethod -Uri 'https://api.github.com/repos/git-for-windows/git/releases/latest' -TimeoutSec 120 } catch { Write-Host 'GitHub API unavailable, using fallback MinGit version' }; \
    $asset = $null; \
    if ($rel) { $asset = $rel.assets | Where-Object { $_.name -match '^MinGit-[0-9\.]+-64-bit\.zip$' } | Select-Object -First 1 }; \
    $url = if ($asset) { $asset.browser_download_url } else { 'https://github.com/git-for-windows/git/releases/download/v2.51.0.windows.2/MinGit-2.51.0.2-64-bit.zip' }; \
    Write-Host "Downloading MinGit from $url"; \
    Invoke-WebRequest -Uri $url -OutFile C:\tools\mingit.zip -TimeoutSec 600; \
    Expand-Archive -Path C:\tools\mingit.zip -DestinationPath C:\tools\mingit -Force; \
    Remove-Item C:\tools\mingit.zip -Force; \
    Write-Host 'MinGit staged at C:\tools\mingit'

# --- carapace completions binary ---------------------------------------------
# Required by the R4 tier tests (spec parity checks invoke carapace probes).
# Assets are versioned: resolve the real name via the GitHub API instead of
# guessing a stable-name release asset (which 404s).
RUN $rel = Invoke-RestMethod -Uri 'https://api.github.com/repos/carapace-sh/carapace-bin/releases/latest' -TimeoutSec 120; \
    $asset = $rel.assets | Where-Object { $_.name -match '^carapace-bin_[0-9\.]+_windows_amd64\.zip$' } | Select-Object -First 1; \
    if ($null -eq $asset) { throw 'no carapace-bin windows_amd64 asset in latest release' }; \
    Write-Host "Downloading $($asset.name)"; \
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile C:\tools\carapace.zip -TimeoutSec 600; \
    Expand-Archive -Path C:\tools\carapace.zip -DestinationPath C:\tools\carapace-bin -Force; \
    $exe = Get-ChildItem C:\tools\carapace-bin -Recurse -Filter carapace.exe | Select-Object -First 1; \
    if ($null -eq $exe) { throw 'carapace.exe missing from archive' }; \
    Move-Item $exe.FullName C:\tools\carapace.exe -Force; \
    Remove-Item C:\tools\carapace.zip -Force; \
    Remove-Item C:\tools\carapace-bin -Recurse -Force; \
    Write-Host 'carapace staged at C:\tools\carapace.exe'

# --- VS BuildTools 2022 with C++ workload (MSVC link.exe + Windows SDK) ------
# Required by the Rust MSVC toolchain that installer\Cargo.toml statically
# links against. Includes the recommended Windows SDK component set.
RUN Invoke-WebRequest -Uri 'https://aka.ms/vs/17/release/vs_buildtools.exe' -OutFile C:\tools\vs_buildtools.exe -TimeoutSec 900; \
    $proc = Start-Process C:\tools\vs_buildtools.exe -ArgumentList @('--quiet', '--wait', '--norestart', '--nocache', '--installPath', 'C:\BuildTools', '--add', 'Microsoft.VisualStudio.Workload.VCTools', '--includeRecommended') -Wait -PassThru; \
    if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 3010) { throw "VS BuildTools install failed with exit code $($proc.ExitCode)" }; \
    Remove-Item C:\tools\vs_buildtools.exe -Force; \
    Write-Host 'VS BuildTools installed at C:\BuildTools'

# --- Rust stable (MSVC default, CARGO_HOME isolated) -------------------------
RUN $env:RUSTUP_HOME = 'C:\rustup'; $env:CARGO_HOME = 'C:\cargo'; \
    Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile C:\tools\rustup-init.exe -TimeoutSec 600; \
    C:\tools\rustup-init.exe -y --default-toolchain stable-msvc --profile minimal --no-modify-path; \
    Remove-Item C:\tools\rustup-init.exe -Force; \
    & C:\cargo\bin\rustc.exe --version

ENV RUSTUP_HOME=C:\rustup \
    CARGO_HOME=C:\cargo \
    RICE_BACKUP_DIR=C:\scratch\.rice-backups

# PATH: pwsh (base) -> MinGit bin -> cargo/rust bin -> carapace
# (container PATH separators are ';')
RUN $add = @('C:\tools\mingit\cmd', 'C:\tools\mingit\bin', 'C:\cargo\bin', 'C:\tools'); \
    $cur = [Environment]::GetEnvironmentVariable('PATH', 'Machine'); \
    [Environment]::SetEnvironmentVariable('PATH', ($add -join ';') + ';' + $cur, 'Machine'); \
    Write-Host 'PATH extended for the container image'

# --- Repository working tree (baked copy; host is never mounted) -------------
WORKDIR C:\repo
COPY . .

LABEL org.opencontainers.image.title="window-rice-testenv" \
      org.opencontainers.image.description="WINDOW-RICE containerized test environment (TEST_INFRA.md #6) - HKDEVS / Window-Rice" \
      org.opencontainers.image.authors="HKDEVS / Project -: Window-Rice"

CMD ["pwsh", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "C:\\repo\\docker\\run-in-container.ps1"]
