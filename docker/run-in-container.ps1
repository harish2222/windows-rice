# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: WINDOW-RICE In-Container Test Suite Executor
# ============================================================================
# Runs INSIDE the window-rice/testenv container (TEST_INFRA.md section 6).
# Execution order:
#   0. cargo build (debug + release) so Select-InstallerExe resolves fresh binaries
#   1. Installer cargo unit suite (33 tests)
#   2. YASB workspace cargo unit suite
#   3. 4-tier E2E suite (tests/e2e/run_all.ps1 -Tier All; 81 scenarios)
# Environment contract honored:
#   RICE_BACKUP_DIR   -> container scratch backup root (wallpaper guard intact)
#   CARGO_TARGET_DIR  -> default (repo-local target inside the throwaway image)
#   USERPROFILE/HOME  -> container profile; sandboxes still virtualize per test
# Exit code: 0 only when every suite reports zero failures.
# Provenance (image id, OS build, UTC timestamp) is stamped per the evidence
# policy so release notes can cite containerized verification.
# ============================================================================

$ErrorActionPreference = "Continue"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host "  WINDOW-RICE CONTAINERIZED TEST RUN (docker/test policy TEST_INFRA#6)" -ForegroundColor Cyan
Write-Host "=====================================================================" -ForegroundColor Cyan

# --- Provenance stamp (evidence policy) --------------------------------------
$imageId = "unknown"
if ($env:IMAGE_ID) { $imageId = $env:IMAGE_ID }
elseif (Test-Path "C:\scratch\image-id.txt") { $imageId = (Get-Content "C:\scratch\image-id.txt" -Raw).Trim() }
$osBuild = [System.Environment]::OSVersion.VersionString
$stamp = "[provenance] image=$imageId os=$osBuild time=$([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')) user=HKDEVS project=Window-Rice"
Write-Host $stamp -ForegroundColor DarkCyan

$hostRepo = "C:\repo"
$failures = 0

# --- 0. Build binaries (debug + release) -------------------------------------
Write-Host "`n>>> [0/3] Building installer binaries (debug + release) <<<" -ForegroundColor Yellow
Push-Location $hostRepo
try {
    $null = & cargo build --manifest-path installer/Cargo.toml 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Debug build FAILED" -ForegroundColor Red
        $failures++
    }
    $null = & cargo build --release --manifest-path installer/Cargo.toml 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Release build FAILED" -ForegroundColor Red
        $failures++
    }
} finally { Pop-Location }

if ($failures -gt 0) {
    Write-Host "container run aborted: builds failed" -ForegroundColor Red
    exit 1
}

# --- 1. Installer unit suite --------------------------------------------------
Write-Host "`n>>> [1/3] Installer cargo unit suite <<<" -ForegroundColor Yellow
Push-Location $hostRepo
try {
    $out = & cargo test --manifest-path installer/Cargo.toml 2>&1
    $exit = $LASTEXITCODE
    $out | Select-String -Pattern "test result:|running \d+ tests|error" | ForEach-Object { Write-Host $_.Line }
    if ($exit -ne 0) {
        Write-Host "Installer unit suite FAILED (exit $exit)" -ForegroundColor Red
        $failures++
    } else {
        Write-Host "Installer unit suite PASSED" -ForegroundColor Green
    }
} finally { Pop-Location }

# --- 2. YASB workspace unit suite ---------------------------------------------
Write-Host "`n>>> [2/3] YASB workspace cargo unit suite <<<" -ForegroundColor Yellow
Push-Location $hostRepo
try {
    $out = & cargo test --manifest-path yasb/Cargo.toml 2>&1
    $exit = $LASTEXITCODE
    $out | Select-String -Pattern "test result:|FAILED" | ForEach-Object { Write-Host $_.Line }
    if ($exit -ne 0 -or ($out -join "`n") -match "FAILED") {
        Write-Host "YASB unit suite FAILED (exit $exit)" -ForegroundColor Red
        $failures++
    } else {
        Write-Host "YASB unit suite PASSED" -ForegroundColor Green
    }
} finally { Pop-Location }

# --- 3. 4-tier E2E suite -------------------------------------------------------
Write-Host "`n>>> [3/3] E2E suite (tiers 1-4) <<<" -ForegroundColor Yellow
Push-Location $hostRepo
try {
    & pwsh -NoProfile -ExecutionPolicy Bypass -File (Join-Path $hostRepo "tests/e2e/run_all.ps1") -Tier All
    if ($LASTEXITCODE -ne 0) {
        Write-Host "E2E suite FAILED (exit $($LASTEXITCODE))" -ForegroundColor Red
        $failures++
    } else {
        Write-Host "E2E suite PASSED" -ForegroundColor Green
    }
} finally { Pop-Location }

$sw.Stop()
Write-Host "`n=====================================================================" -ForegroundColor Cyan
if ($failures -eq 0) {
    Write-Host "CONTAINER TEST RUN: ALL SUITES PASSED" -ForegroundColor Green
} else {
    Write-Host "CONTAINER TEST RUN: $failures SUITE(S) FAILED" -ForegroundColor Red
}
Write-Host ("Elapsed: {0}s" -f [math]::Round($sw.Elapsed.TotalSeconds, 1)) -ForegroundColor Gray
Write-Host $stamp -ForegroundColor DarkCyan

if ($failures -gt 0) { exit 1 }
exit 0
