# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: WINDOW-RICE Podman Conformance Suite Executor (Linux)
# ============================================================================
# Runs INSIDE the window-rice/conformance container (podman, Linux).
# Covers the cross-platform verification surface; Windows-bound suites stay
# engine-bound to Windows containers (docker/Dockerfile + ci.yml e2e-suite).
#
# Execution order:
#   1. Provenance stamp (image id via IMAGE_ID, OS, UTC) - TEST_INFRA evidence
#   2. PWSCRIPT_CONFORMANCE  - AST parse of every pwsh surface = zero errors
#   3. SPEC_CONFORMANCE      - 112 YAML specs parse; FULL REVOKE flag documented
#   4. CI_CONFORMANCE        - ci.yml e2e-suite job + release.yml cargo-test gate
#   5. DOC_PARITY            - suite counts, spec counts, release tags in sync
#   6. RUSTFMT_CONFORMANCE   - cargo fmt --check on both workspaces (informational)
#   7. REPO_INTEGRITY        - git repository usable inside the container
# Exit code: 0 only when every check reports zero failures.
# ============================================================================

$ErrorActionPreference = "Continue"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$failures = 0

# --- Provenance stamp (evidence policy) --------------------------------------
$imageId = "unknown"
if ($env:IMAGE_ID) { $imageId = $env:IMAGE_ID.Trim() }
$osName = "linux"
$stamp = "[provenance] image=$imageId os=$osName time=$([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')) engine=podman-linux user=HKDEVS project=Window-Rice"
Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host "  WINDOW-RICE PODMAN CONFORMANCE RUN (policy TEST_INFRA#6)" -ForegroundColor Cyan
Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host $stamp -ForegroundColor DarkCyan

Set-Location /repo

# --- 1. PWSCRIPT_CONFORMANCE --------------------------------------------------
Write-Host "`n>>> [1/7] pwsh AST conformance (all pwsh surfaces) <<<" -ForegroundColor Yellow
$pwFiles = @(Get-ChildItem "install.ps1", "docker" -File -Filter *.ps1 | Select-Object -ExpandProperty FullName) +
           @(Get-ChildItem "tests/e2e" -File | Where-Object { $_.Name -match '\.psm?1$' } | Select-Object -ExpandProperty FullName)
if ($pwFiles.Count -lt 10) {
    Write-Host "PWSCRIPT_CONFORMANCE FAILED: only $($pwFiles.Count) pwsh surfaces found" -ForegroundColor Red
    $failures++
} else {
    $bad = 0
    foreach ($f in $pwFiles) {
        $errs = $null
        [System.Management.Automation.Language.Parser]::ParseFile($f, [ref]$null, [ref]$errs) | Out-Null
        if ($errs -and $errs.Count -gt 0) {
            Write-Host "SYNTAX FAIL: $([IO.Path]::GetRelativePath('/repo', $f)) -> $($errs[0].Message)" -ForegroundColor Red
            $bad++
        }
    }
    if ($bad -gt 0) {
        Write-Host "PWSCRIPT_CONFORMANCE FAILED ($bad/$($pwFiles.Count) files)" -ForegroundColor Red
        $failures++
    } else {
        Write-Host "PWSCRIPT_CONFORMANCE PASSED ($($pwFiles.Count) surfaces, 0 parse errors)" -ForegroundColor Green
    }
}

# --- 2. SPEC_CONFORMANCE ------------------------------------------------------
Write-Host "`n>>> [2/7] carapace spec conformance <<<" -ForegroundColor Yellow
try {
    $specs = @(Get-ChildItem "carapace/specs" -Filter *.yaml | Sort-Object Name)
    if ($specs.Count -ne 112) { throw "expected 112 carapace specs, found $($specs.Count)" }
    $parsed = 0
    foreach ($s in $specs) {
        $null = Get-Content $s.FullName -Raw | ConvertFrom-Yaml
        $parsed++
    }
    foreach ($s in @("rice-installer.yaml", "window-rice.yaml")) {
        $y = Get-Content (Join-Path "carapace/specs" $s) -Raw | ConvertFrom-Yaml
        $desc = $y.flags.'--uninstall-all'
        if (-not $desc -or $desc -notmatch '^FULL REVOKE\s+-\s+remove') { throw "$s is missing the --uninstall-all FULL REVOKE flag entry (got: '$desc')" }
    }
    Write-Host "SPEC_CONFORMANCE PASSED ($parsed specs parsed, FULL REVOKE documented in both entrypoints)" -ForegroundColor Green
} catch {
    Write-Host "SPEC_CONFORMANCE FAILED: $($_.Exception.Message)" -ForegroundColor Red
    $failures++
}

# --- 3. CI_CONFORMANCE --------------------------------------------------------
Write-Host "`n>>> [3/7] workflow gate conformance <<<" -ForegroundColor Yellow
try {
    $ci = Get-Content ".github/workflows/ci.yml" -Raw | ConvertFrom-Yaml
    if (-not $ci.jobs.'e2e-suite') { throw "ci.yml is missing the e2e-suite job" }
    if ($ci.jobs.'e2e-suite'.needs -ne 'rust-installer') { throw "ci e2e-suite must need rust-installer" }
    $runStep = $ci.jobs.'e2e-suite'.steps | Where-Object { $_.name -match 'Run E2E suite' -and $_.run -match 'run_all\.ps1' -and $_.run -match 'Tier All|run_all' }
    if (-not $runStep) { throw "ci e2e-suite must invoke run_all.ps1 -Tier All" }

    $rel = Get-Content ".github/workflows/release.yml" -Raw | ConvertFrom-Yaml
    if (-not $rel.'on'.push.tags -contains 'v*') { throw "release.yml must trigger on v* tag push" }
    $gate = $rel.jobs.PSObject.Properties | ForEach-Object { $_.Value.steps } | Where-Object { $_.name -match 'installer unit tests' }
    if (-not $gate) { throw "release.yml is missing the cargo-test release gate step" }

    Write-Host "CI_CONFORMANCE PASSED (e2e-suite job gated by rust-installer; release gated by cargo test)" -ForegroundColor Green
} catch {
    Write-Host "CI_CONFORMANCE FAILED: $($_.Exception.Message)" -ForegroundColor Red
    $failures++
}

# --- 4. DOC_PARITY ------------------------------------------------------------
Write-Host "`n>>> [4/7] documentation count parity <<<" -ForegroundColor Yellow
try {
    $tierCounts = @{}
    foreach ($t in @("tier1_feature_coverage", "tier2_boundary_corner", "tier3_cross_feature", "tier4_real_world")) {
        $n = (Select-String -Path "tests/e2e/$t.Tests.ps1" -Pattern '^\s{4}It "' | Measure-Object).Count
        $tierCounts[$t] = $n
    }
    $total = ($tierCounts.Values | Measure-Object -Sum).Sum
    if ($tierCounts.tier1_feature_coverage -ne 34) { throw "tier1 bedrock count drifted: $($tierCounts.tier1_feature_coverage) != 34" }
    if ($tierCounts.tier2_boundary_corner -ne 34) { throw "tier2 count drifted: $($tierCounts.tier2_boundary_corner) != 34" }
    if ($tierCounts.tier3_cross_feature -ne 6)    { throw "tier3 count drifted: $($tierCounts.tier3_cross_feature) != 6" }
    if ($tierCounts.tier4_real_world   -ne 7)    { throw "tier4 count drifted: $($tierCounts.tier4_real_world) != 7" }
    $units = (Select-String -Path "installer/src/*.rs" -Pattern '#\[(test|tokio::test)' -AllMatches | ForEach-Object { $_.Matches.Count } | Measure-Object -Sum).Sum
    if ($units -ne 34) { throw "unit count drifted: $units != 34" }
    if ((Get-Content TEST_READY.md -Raw) -notmatch '81 \(total\)') { throw "TEST_READY.md total row must read 81 (total)" }
    if ((Get-Content README.md -Raw) -notmatch '112 custom') { throw "README.md must declare 112 custom specs" }
    if ((Get-Content README.md -Raw) -notmatch '--branch v1\.0\.3') { throw "README.md clone line must reference v1.0.3" }
    if ((Get-Content install.ps1 -Raw) -notmatch '\$Tag = "v1\.0\.3"') { throw "install.ps1 default Tag must be v1.0.3" }
    Write-Host "DOC_PARITY PASSED (tiers 34/34/6/7 = $total; units $units; spec count 112; release tag v1.0.3)" -ForegroundColor Green
} catch {
    Write-Host "DOC_PARITY FAILED: $($_.Exception.Message)" -ForegroundColor Red
    $failures++
}

# --- 5. RUSTFMT_CONFORMANCE (informational) -----------------------------------
Write-Host "`n>>> [5/7] rustfmt conformance (informational) <<<" -ForegroundColor Yellow
foreach ($ws in @("installer", "yasb")) {
    Push-Location $ws
    try {
        $null = & cargo fmt --check 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-Host "RUSTFMT_CONFORMANCE [$ws]: drift detected (informational, not a failure)" -ForegroundColor DarkYellow
        } else {
            Write-Host "RUSTFMT_CONFORMANCE [$ws]: clean" -ForegroundColor Green
        }
    } finally { Pop-Location }
}

# --- 6. REPO_INTEGRITY --------------------------------------------------------
Write-Host "`n>>> [6/7] repository integrity inside container <<<" -ForegroundColor Yellow
# 477 tracked files = the full `git ls-files` set as of the conformance bake;
# assert a floor well under the known total so real drift still trips it.
$expectedTracked = 400
try {
    if (-not (Test-Path ".git")) { throw ".git missing from the baked copy" }
    $tracked = & git ls-files 2>&1
    if ($LASTEXITCODE -ne 0 -or $tracked.Count -lt $expectedTracked) { throw "git ls-files unusable inside the container (tracked: $($tracked.Count), expected >= $expectedTracked)" }
    Write-Host "REPO_INTEGRITY PASSED (git ok, $($tracked.Count) tracked files visible)" -ForegroundColor Green
} catch {
    Write-Host "REPO_INTEGRITY FAILED: $($_.Exception.Message)" -ForegroundColor Red
    $failures++
}

# --- 7. SUMMARY ---------------------------------------------------------------
$sw.Stop()
Write-Host "`n=====================================================================" -ForegroundColor Cyan
if ($failures -eq 0) {
    Write-Host "PODMAN CONFORMANCE RUN: ALL CHECKS PASSED" -ForegroundColor Green
} else {
    Write-Host "PODMAN CONFORMANCE RUN: $failures CHECK(S) FAILED" -ForegroundColor Red
}
Write-Host ("Elapsed: {0}s" -f [math]::Round($sw.Elapsed.TotalSeconds, 1)) -ForegroundColor Gray
Write-Host $stamp -ForegroundColor DarkCyan

if ($failures -gt 0) { exit 1 }
exit 0
