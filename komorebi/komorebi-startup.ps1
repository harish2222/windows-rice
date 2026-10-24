# Komorebi startup script (single startup owner; runs hidden via Startup .lnk)
# Starts Komorebi once, waits briefly for its IPC endpoint, then starts YASB
# and the selected desktop apps.
param([switch]$NoApps)

$ErrorActionPreference = 'SilentlyContinue'

# 1. Start Komorebi if it is not already running.
if (-not (Get-Process -Name komorebi -ErrorAction SilentlyContinue)) {
    $null = komorebic start --config "$env:USERPROFILE\.config\komorebi\komorebi.json" 2>&1
}

# 2. Wait for Komorebi IPC, checking every 250 ms (up to 5 seconds).
$ready = $false
for ($i = 0; $i -lt 20; $i++) {
    $null = komorebic state 2>&1
    if ($LASTEXITCODE -eq 0) { $ready = $true; break }
    Start-Sleep -Milliseconds 250
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

    # 5. Workspace 2 (index 1) -> Zen Browser (also its initial-workspace rule)
    $null = komorebic focus-monitor-workspace 0 1 2>&1
    $zen = "$env:LOCALAPPDATA\Zen Browser\zen.exe"
    if (-not (Test-Path -LiteralPath $zen)) {
        $zen = "C:\Users\haris\AppData\Local\ZENBRO~1\zen.exe"
    }
    Start-Process -FilePath $zen
    Start-Sleep -Seconds 2

    # 6. Return to workspace 1
    $null = komorebic focus-monitor-workspace 0 0 2>&1
}
