# Komorebi startup script v2 (runs via Startup .lnk with -NoProfile)
# Ownership:
#   - komorebi.lnk (official `komorebic enable-autostart`) starts komorebi
#   - this script: guarded fallback start + wait-until-ready + YASB + apps
# Fixes vs v1: removed `komorebic stop` (errored when no instance was running
# and raced the start guard), removed profile load (threw on every spawn).
param([switch]$NoApps)

$ErrorActionPreference = 'SilentlyContinue'

# 1. Fallback: start komorebi only if the official autostart did not
if (-not (Get-Process -Name komorebi -ErrorAction SilentlyContinue)) {
    $null = komorebic start --config "$env:USERPROFILE\.config\komorebi\komorebi.json" 2>&1
}

# 2. Wait until komorebi responds (up to 30s)
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
    $null = komorebic state 2>&1
    if ($LASTEXITCODE -eq 0) { $ready = $true; break }
    Start-Sleep -Seconds 1
}
if (-not $ready) { exit 1 }

# 3. Ensure YASB bar is running (guarded; glazewm also shells it at startup)
if (-not (Get-Process -Name yasb -ErrorAction SilentlyContinue)) {
    Start-Process yasb
}

if (-not $NoApps) {
    # 4. Workspace 1 (index 0) -> Windows Terminal
    $null = komorebic focus-monitor-workspace 0 0 2>&1
    Start-Process wt.exe
    Start-Sleep -Milliseconds 1500

    # 5. Workspace 2 (index 1) -> Zen Browser (also its initial-workspace rule),
    #    maximized for video/fullscreen use
    $null = komorebic focus-monitor-workspace 0 1 2>&1
    $zen = "$env:LOCALAPPDATA\Zen Browser\zen.exe"
    if (-not (Test-Path -LiteralPath $zen)) {
        $zen = "C:\Users\haris\AppData\Local\ZENBRO~1\zen.exe"
    }
    Start-Process -FilePath $zen
    Start-Sleep -Seconds 2
    $null = komorebic focus-monitor-workspace 0 1 2>&1
    $null = komorebic toggle-maximize 2>&1

    # 6. Return to workspace 1
    $null = komorebic focus-monitor-workspace 0 0 2>&1
}
