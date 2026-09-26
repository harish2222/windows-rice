# Komorebi startup script
# - Starts Komorebi if not already running
# - Ensures YASB bar is running (single guarded owner; GlazeWM/HKCU entries removed)
# - Opens a terminal on workspace 1 (index 0), unless -NoApps
# - Opens Zen Browser fullscreen on workspace 2 (index 1), unless -NoApps
param([switch]$NoApps)

$ErrorActionPreference = 'SilentlyContinue'

# 1. Ensure Komorebi is running
if (-not (Get-Process -Name komorebi -ErrorAction SilentlyContinue)) {
    komorebic start | Out-Null
}

# 1b. Ensure YASB bar is running (sole guarded owner of the bar at boot)
if (-not (Get-Process -Name yasb -ErrorAction SilentlyContinue)) {
    Start-Process yasb
}

# 2. Wait until Komorebi is responsive
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
    komorebic state > $null 2>&1
    if ($LASTEXITCODE -eq 0) { $ready = $true; break }
    Start-Sleep -Seconds 1
}
if (-not $ready) { exit }

if (-not $NoApps) {
    # 3. Workspace 1 (index 0) -> Windows Terminal
    komorebic focus-monitor-workspace 0 0 | Out-Null
    Start-Process wt.exe
    Start-Sleep -Seconds 2

    # 4. Workspace 2 (index 1) -> Zen Browser (fullscreen)
    komorebic focus-monitor-workspace 0 1 | Out-Null
    Start-Process -FilePath "C:\Users\haris\AppData\Local\Zen Browser\zen.exe"
    Start-Sleep -Seconds 3
    komorebic focus-monitor-workspace 0 1 | Out-Null
    komorebic toggle-maximize | Out-Null

    # 5. Return to workspace 1
    komorebic focus-monitor-workspace 0 0 | Out-Null
}
