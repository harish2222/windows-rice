# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Restore Previous Script Execution Policy (EXEC-POLICY-WIDEN fix)
# ============================================================================
# Restores the ExecutionPolicy value that WINDOW-RICE install.ps1 recorded in
# .config\.rice\execution-policy-previous.txt before it switched CurrentUser
# scope to RemoteSigned. Companion to the v1.0.4/1.0.5 security cycle
# (review run ed156b73, finding EXEC-POLICY-WIDEN).
# Usage:  pwsh -NoProfile -ExecutionPolicy Bypass -File .\tools\restore-exec-policy.ps1
#         (or: tools\restore-exec-policy.ps1 -Show to only print)
# ============================================================================

[CmdletBinding()]
param(
    [switch]$Show
)

$statePath = Join-Path $env:USERPROFILE ".config\.rice\execution-policy-previous.txt"

if (-not (Test-Path $statePath)) {
    Write-Host "[Windows Rice] No recorded previous policy found at: $statePath" -ForegroundColor Yellow
    exit 1
}

$priorPolicy = (Get-Content $statePath -Raw).Trim()
Write-Host "[Windows Rice] Recorded previous ExecutionPolicy (CurrentUser): $priorPolicy" -ForegroundColor Cyan

if ($Show) {
    Write-Host ("[Windows Rice] Current ExecutionPolicy (CurrentUser): " + (Get-ExecutionPolicy -Scope CurrentUser))
    Write-Host ("[Windows Rice] Current effective ExecutionPolicy: " + (Get-ExecutionPolicy))
    exit 0
}

Set-ExecutionPolicy -ExecutionPolicy $priorPolicy -Scope CurrentUser -Force
Write-Host "[Windows Rice] ExecutionPolicy restored to '$priorPolicy' (CurrentUser scope)." -ForegroundColor Green
