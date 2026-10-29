# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: WINDOW-RICE Host-Side Container Verification Runner
# ============================================================================
# TEST_INFRA.md section 6 - container-only verification policy (host exec = 0).
# USAGE (from repo root):
#   pwsh -NoProfile -ExecutionPolicy Bypass -File docker/run-rice-tests.ps1
# Steps:
#   1. Preflight: Docker engine reachable AND OSType == windows.
#      (Windows containers are mandatory: MSVC build + Windows junctions +
#      profile isolation. podman-on-Windows only runs Linux containers.)
#   2. Build window-rice/testenv image from this repo (context = repo root
#      so C:\repo is a BAKED copy; the live working tree is never mounted).
#   3. docker run -e IMAGE_ID=<id> window-rice/testenv
#      All suites (builds, 33 cargo units, YASB units, 81 E2E) run in-container
#      and the container exit code becomes this script's exit code.
# The live master desktop config is never touched: no mount, no host exec.
# ============================================================================

param(
    [string]$ImageName = "window-rice/testenv",
    [switch]$NoRebuild
)

$ErrorActionPreference = "Stop"

# --- Preflight: engine + OSType ------------------------------------------------
Write-Host "[preflight] checking container engine..." -ForegroundColor Cyan
$info = docker info --format "{{.OSType}}|{{.OperatingSystem}}" 2>$null
if (-not $info) {
    Write-Host "<prebuilt error> :: Docker engine is not reachable." -ForegroundColor Red
    Write-Host "Start Docker Desktop, then switch the engine to Windows containers (tray icon -> 'Switch to Windows containers...' or DockerCli.exe -SwitchWindowsEngine)." -ForegroundColor Yellow
    exit 2
}
$ostype = ($info -split '\|')[0]
if ($ostype -ne "windows") {
    Write-Host "<prebuilt error> :: Docker engine OSType is '$ostype' - Windows containers required." -ForegroundColor Red
    Write-Host "Switch the engine to Windows containers (tray icon -> 'Switch to Windows containers...' or run DockerCli.exe -SwitchWindowsEngine from Docker Desktop's program folder)." -ForegroundColor Yellow
    Write-Host "Note: podman cannot serve these test suites: podman-on-Windows runs Linux containers only." -ForegroundColor Yellow
    exit 2
}
Write-Host "[preflight] engine OK (Windows containers) - OS: $($info)" -ForegroundColor Green

# --- Build (repo root as context; docker/Dockerfile baked repo at C:\repo) -----
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$cidfile = Join-Path $env:TEMP "window-rice-test-run.cid"

if (-not $NoRebuild) {
    Write-Host "`n[build] building $ImageName (first build installs VS BuildTools + Rust: expect 20-40 min)..." -ForegroundColor Cyan
    docker build -f (Join-Path $PSScriptRoot "Dockerfile") -t $ImageName $repoRoot
    if ($LASTEXITCODE -ne 0) {
        Write-Host "<prebuilt error> :: image build failed (exit $LASTEXITCODE)." -ForegroundColor Red
        exit 3
    }
} else {
    Write-Host "[build] -NoRebuild: reusing existing $ImageName image" -ForegroundColor Yellow
}

$imageId = (docker image inspect $ImageName --format "{{.Id}}").Trim()
Write-Host "[run] provenance image id: $imageId" -ForegroundColor DarkCyan

# --- Execute all suites inside the container -----------------------------------
Write-Host "`n[run] executing all suites inside the container (no host exec)..." -ForegroundColor Cyan
docker run --rm --cidfile $cidfile -e IMAGE_ID="$imageId" $ImageName
$exit = $LASTEXITCODE

if (Test-Path $cidfile) {
    $cid = (Get-Content $cidfile -Raw).Trim()
    $summary = docker logs $cid 2>$null | Select-String -Pattern 'provenance|PASSED|FAILED|SUITE'
    $summary | ForEach-Object { Write-Host $_.Line }
    Remove-Item $cidfile -Force -ErrorAction SilentlyContinue
}

Write-Host "`n[run] container run exit code: $exit" -ForegroundColor $(if ($exit -eq 0) { 'Green' } else { 'Red' })
exit $exit
