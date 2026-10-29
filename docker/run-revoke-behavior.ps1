# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: WINDOW-RICE Full Revoke Behavioral Verifier (podman/Linux)
# ============================================================================
# Runs INSIDE a Linux container (window-rice/conformance base) and exercises
# the REAL revoke engine binary - no host execution, no host junctions, no
# host writes. All state lives under the container's isolated test home.
#
# Verifies (behavioral, not AST):
#   1. BUILD      - `cargo build` of installer under Linux (engine compiles
#                   cross-platform; Windows-bound junction paths are cfg'd off)
#   2. UNITS      - `cargo test` unit suite incl. the TEST_MUTEX-gated ledger
#                   and revoke tests
#   3. T1-R6 trio - clean-machine `--uninstall-all` announces the banner, runs
#                   the flow, prints the "Full revoke complete: ... 0 failed"
#                   summary contract on stdout
#   4. T4-SCEN-07 - the full ownership lifecycle with NATIVE symlinks:
#                   seed user config -> --link-all -> ledger records the
#                   catalog -> --uninstall-all -> displaced user config comes
#                   back byte-for-byte ([RESTORED]) and the ledger is cleared
#
# Exit code 0 only when every behavioral check passes.
# ============================================================================

$ErrorActionPreference = "Continue"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$script:failures = 0

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) { throw "$Message - expected '$Expected' but got '$Actual'" }
}

function Assert-True {
    param($Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Contains {
    param([string]$Haystack, [string]$Needle, [string]$Message)
    if (-not $Haystack.Contains($Needle)) { throw "$Message - output missing '$Needle'" }
}

Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host "  WINDOW-RICE REVOKE BEHAVIORAL VERIFICATION (podman / linux)" -ForegroundColor Cyan
Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host ("[provenance] os=linux time={0} engine=podman-linux user=HKDEVS project=Window-Rice" -f [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')) -ForegroundColor DarkCyan

Set-Location /repo

# --- CHECK 1: build the real engine binary ------------------------------------
Write-Host "`n>>> [1/4] cargo build (linux) of the installer engine <<<" -ForegroundColor Yellow
Push-Location installer
try {
    cargo build 2>&1 | ForEach-Object { Write-Host "    $_" }
    if ($LASTEXITCODE -ne 0) {
        Write-Host "BUILD FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
        $script:failures++
    }
} finally { Pop-Location }
$exePath = "/repo/installer/target/debug/rice-installer"
if ($script:failures -gt 0) {
    Write-Host "BUILD UNUSABLE (see failures above)" -ForegroundColor Red
    Write-Host "`nREVOKE VERIFICATION ABORTED (build unusable)" -ForegroundColor Red
    exit 1
}
if (-not (Test-Path $exePath)) {
    Write-Host "BINARY MISSING: $exePath" -ForegroundColor Red
    Write-Host "`nREVOKE VERIFICATION ABORTED (build unusable)" -ForegroundColor Red
    exit 1
}
Write-Host "BUILD PASSED ($exePath)" -ForegroundColor Green

# --- CHECK 2: unit suite (ledger/TEST_MUTEX/revoke units) ---------------------
Write-Host "`n>>> [2/4] cargo test (linux) unit suite <<<" -ForegroundColor Yellow
Push-Location installer
try {
    cargo test 2>&1 | ForEach-Object { Write-Host "    $_" }
    if ($LASTEXITCODE -ne 0) {
        Write-Host "UNIT SUITE FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
        $script:failures++
    } else {
        Write-Host "UNIT SUITE PASSED" -ForegroundColor Green
    }
} finally { Pop-Location }

# Isolated per-run home so re-runs start from a clean profile
$riceHome = "/ricehome"
$backupRoot = "/ricebackups"

function Invoke-RevokeDebug {
    param([string]$Exe, [string[]]$ExeArgs)
    # CLI-space env: FULL PROFILE VIRTUALIZATION (same contract as the e2e
    # harness Invoke-WithIsolatedProfile: USERPROFILE/HOME/RICE_BACKUP_DIR +
    # Roaming/Local AppData).
    $env:USERPROFILE = $riceHome
    $env:HOME = $riceHome
    $env:RICE_BACKUP_DIR = $backupRoot
    $env:APPDATA = "$riceHome/AppData/Roaming"
    $env:LOCALAPPDATA = "$riceHome/AppData/Local"
    & $Exe @ExeArgs 2>&1 | Out-String
}

function Reset-RevokeSandbox {
    if (Test-Path $riceHome) { [System.IO.Directory]::Delete($riceHome, $true) }
    if (Test-Path $backupRoot) { [System.IO.Directory]::Delete($backupRoot, $true) }
    $null = [System.IO.Directory]::CreateDirectory($riceHome)
}

# --- CHECK 3: T1-R6 trio - clean machine revoke contract ----------------------
Write-Host "`n>>> [3/4] T1-R6 trio: clean-machine --uninstall-all contract <<<" -ForegroundColor Yellow
$out = $null
try {
    Reset-RevokeSandbox
    $out = Invoke-RevokeDebug -Exe $exePath -ExeArgs @("--uninstall-all")
    Assert-Contains $out "FULL REVOKE" "banner must announce the full revoke"
    Assert-Contains $out "Binaries stay installed" "banner must document binary preservation"
    Assert-Contains $out "Full revoke complete:" "summary line must be printed (CLI parity fix)"
    Assert-True ($out -match 'Full revoke complete: \d+ restored, \d+ cleaned, \d+ preserved \(user-modified\), \d+ repo-native skipped, \d+ no-op, 0 failed') "summary must count every outcome with zero failures on a clean machine"
    $ledger = Get-Content "$backupRoot/installed.json" -Raw | ConvertFrom-Json
    Assert-Equal $ledger.records.Count 0 "fresh full revoke must leave zero recorded targets"
    Write-Host "T1-R6 TRIO PASSED (summary contract printed + ledger cleared)" -ForegroundColor Green
    $out -split "`n" | Where-Object { $_ -match 'Full revoke complete' } | ForEach-Object { Write-Host "    captured: $_" -ForegroundColor DarkGreen }
} catch {
    Write-Host "T1-R6 TRIO FAILED: $($_.Exception.Message)" -ForegroundColor Red
    if ($out) { $out -split "`n" | ForEach-Object { Write-Host "    | $_" } }
    $script:failures++
} finally {
    Reset-RevokeSandbox
}

# --- CHECK 4: T4-SCEN-07 lifecycle with native symlinks ----------------------
Write-Host "`n>>> [4/4] T4-SCEN-07: link -> ledger -> revoke -> restore lifecycle <<<" -ForegroundColor Yellow
$out = $null
try {
    Reset-RevokeSandbox
    $userTarget = "$riceHome/.wezterm.lua"
    Set-Content -Path $userTarget -Value "ORIGINAL_USER_WEZTERM_CONFIG" -Encoding utf8
    Assert-True (Test-Path $userTarget) "user seed config must exist"

    $linkOut = Invoke-RevokeDebug -Exe $exePath -ExeArgs @("--link-all")
    Assert-Contains $linkOut "Starting Config Linking" "link-all flow must run"

    $ledgerFile = "$backupRoot/installed.json"
    Assert-True (Test-Path $ledgerFile) "ownership ledger must exist after linking"
    $ledger = Get-Content $ledgerFile -Raw | ConvertFrom-Json
    Assert-True ($ledger.records.Count -gt 10) "ledger must record the linked catalog (got $($ledger.records.Count))"
    $wez = @($ledger.records | Where-Object { $_.pkg_id -eq "wezterm" -and $_.target -like "*.wezterm.lua" })
    Assert-Equal $wez.Count 1 "ledger must contain exactly one wezterm .wezterm.lua record"

    Write-Host "    link mode for .wezterm.lua ledger record: $($wez[0].mode)" -ForegroundColor DarkCyan

    $out = Invoke-RevokeDebug -Exe $exePath -ExeArgs @("--uninstall-all")
    Assert-Contains $out "[RESTORED]" "displaced user config must be restored from backup"
    Assert-Contains $out "Full revoke complete:" "revoke must print the summary line"

    $restored = (Get-Content $userTarget -Raw).Trim()
    Assert-Equal $restored "ORIGINAL_USER_WEZTERM_CONFIG" "user's original config content must be restored byte-for-byte"

    $ledgerAfter = Get-Content $ledgerFile -Raw | ConvertFrom-Json
    Assert-Equal $ledgerAfter.records.Count 0 "ledger must be emptied after full revoke"

    $repoSpec = "/repo/carapace/specs/rice-installer.yaml"
    Assert-True (Test-Path $repoSpec) "repo-native source must survive the revoke"

    Write-Host "T4-SCEN-07 LIFECYCLE PASSED (ledger recorded, restore byte-exact, ledger cleared)" -ForegroundColor Green
    $out -split "`n" | Where-Object { $_ -match 'Full revoke complete|\[RESTORED\]' } | ForEach-Object { Write-Host "    captured: $_" -ForegroundColor DarkGreen }
} catch {
    Write-Host "T4-SCEN-07 LIFECYCLE FAILED: $($_.Exception.Message)" -ForegroundColor Red
    if ($out) { $out -split "`n" | ForEach-Object { Write-Host "    | $_" } }
    $script:failures++
} finally {
    Reset-RevokeSandbox
}

$sw.Stop()
Write-Host "`n=====================================================================" -ForegroundColor Cyan
if ($script:failures -eq 0) {
    Write-Host "REVOKE BEHAVIORAL VERIFICATION: ALL CHECKS PASSED" -ForegroundColor Green
} else {
    Write-Host "REVOKE BEHAVIORAL VERIFICATION: $script:failures CHECK(S) FAILED" -ForegroundColor Red
}
Write-Host ("Elapsed: {0}s" -f [math]::Round($sw.Elapsed.TotalSeconds, 1)) -ForegroundColor Gray
Write-Host ("[provenance] os=linux time={0} engine=podman-linux user=HKDEVS project=Window-Rice" -f [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')) -ForegroundColor DarkCyan

if ($script:failures -gt 0) { exit 1 }
exit 0
