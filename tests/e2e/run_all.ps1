# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: E2E Test Suite Master Orchestrator and Runner
# ============================================================================

[CmdletBinding()]
param(
    [string]$Tier = "All"
)

$ErrorActionPreference = "Stop"
$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Import-Module (Join-Path $ScriptRoot "Common.psm1") -Force

Write-Host @"
================================================================================
           WINDOWS RICE END-TO-END (E2E) TEST SUITE ORCHESTRATOR               
================================================================================
Repository Root : $(Resolve-Path (Join-Path $ScriptRoot "..\.."))
Runner Mode     : Opaque-Box Requirement-Driven Multi-Tier Suite
Target Tier     : $Tier
Execution Time  : $(Get-Date -Format "yyyy-MM-dd HH:mm:ss UTC")
================================================================================
"@ -ForegroundColor Cyan

Reset-TestResults
$overallTimer = [System.Diagnostics.Stopwatch]::StartNew()

$tierFiles = @(
    @{ Name = "Tier 1"; Path = Join-Path $ScriptRoot "tier1_feature_coverage.Tests.ps1" },
    @{ Name = "Tier 2"; Path = Join-Path $ScriptRoot "tier2_boundary_corner.Tests.ps1" },
    @{ Name = "Tier 3"; Path = Join-Path $ScriptRoot "tier3_cross_feature.Tests.ps1" },
    @{ Name = "Tier 4"; Path = Join-Path $ScriptRoot "tier4_real_world.Tests.ps1" }
)

foreach ($tf in $tierFiles) {
    if ($Tier -ne "All" -and $Tier -ne $tf.Name) {
        continue
    }

    Write-Host "`n>>> RUNNING $($tf.Name) <<<" -ForegroundColor Yellow
    if (Test-Path $tf.Path) {
        . $tf.Path
    } else {
        Write-Host "WARNING: Test file not found: $($tf.Path)" -ForegroundColor Red
    }
}

$overallTimer.Stop()
$summary = Get-TestSummary

Write-Host @"

================================================================================
                           E2E TEST EXECUTION SUMMARY                           
================================================================================
"@ -ForegroundColor Cyan

Write-Host "Tier Breakdown:" -ForegroundColor DarkCyan
Write-Host ("{0,-15} {1,8} {2,8} {3,8} {4,10}" -f "Tier", "Total", "Passed", "Failed", "Pass Rate") -ForegroundColor Gray
Write-Host ("-" * 53) -ForegroundColor Gray
foreach ($t in $summary.ByTier) {
    $rate = if ($t.Total -gt 0) { [math]::Round(($t.Passed / $t.Total) * 100, 1) } else { 0 }
    $color = if ($t.Failed -eq 0) { "Green" } else { "Red" }
    Write-Host ("{0,-15} {1,8} {2,8} {3,8} {4,9}%" -f $t.Tier, $t.Total, $t.Passed, $t.Failed, $rate) -ForegroundColor $color
}

Write-Host "`nFeature Breakdown:" -ForegroundColor DarkCyan
Write-Host ("{0,-50} {1,8} {2,8} {3,8}" -f "Feature Area", "Total", "Passed", "Failed") -ForegroundColor Gray
Write-Host ("-" * 78) -ForegroundColor Gray
foreach ($f in $summary.ByFeature) {
    $color = if ($f.Failed -eq 0) { "Green" } else { "Red" }
    Write-Host ("{0,-50} {1,8} {2,8} {3,8}" -f $f.Feature, $f.Total, $f.Passed, $f.Failed) -ForegroundColor $color
}

$failedColor = if ($summary.Failed -eq 0) { "Green" } else { "Red" }
Write-Host ("=" * 80) -ForegroundColor Cyan
Write-Host ("TOTAL TESTS : {0}" -f $summary.Total) -ForegroundColor White
Write-Host ("PASSED      : {0}" -f $summary.Passed) -ForegroundColor Green
Write-Host ("FAILED      : {0}" -f $summary.Failed) -ForegroundColor $failedColor
Write-Host ("ELAPSED     : {0}s" -f [math]::Round($overallTimer.Elapsed.TotalSeconds, 2)) -ForegroundColor Gray
Write-Host ("=" * 80) -ForegroundColor Cyan

if ($summary.Failed -gt 0) {
    Write-Host "`nFAILURES DETECTED:" -ForegroundColor Red
    $results = Get-TestResults
    foreach ($r in ($results | Where-Object { -not $_.Passed })) {
        Write-Host "  - [$($r.Tier)] [$($r.Feature)] $($r.Name)" -ForegroundColor Red
        Write-Host "    $($r.Error)" -ForegroundColor DarkRed
    }
    Write-Host "`nE2E SUITE STATUS: FAILED" -ForegroundColor Red
    exit 1
} else {
    Write-Host "`nE2E SUITE STATUS: ALL TESTS PASSED [100% VERIFIED]" -ForegroundColor Green
    exit 0
}
