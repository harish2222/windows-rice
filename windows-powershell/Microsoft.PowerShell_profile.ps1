### HKDEVS PowerShell 5.1 Profile - Carapace + Oh-My-Posh
$env:CARAPACE_BRIDGES = 'zsh,fish,bash,inshellisense'

# Carapace completions (official setup) — init script cached so the carapace
# process isn't spawned on every startup (regenerated when carapace.exe changes).
if (Test-Path 'C:\Users\haris\scoop\apps\carapace\current\carapace.exe') {
    Set-Alias -Name carapace -Value 'C:\Users\haris\scoop\apps\carapace\current\carapace.exe' -Force
    Set-PSReadLineOption -Colors @{ "Selection" = "`e[7m" }
    Set-PSReadlineKeyHandler -Key Tab -Function MenuComplete

    $carapaceCacheDir = Join-Path $env:LOCALAPPDATA 'carapace'
    $carapaceCache = Join-Path $carapaceCacheDir 'init-windows-powershell-cached.ps1'
    $carapaceCacheItem = Get-Item -LiteralPath $carapaceCache -ErrorAction SilentlyContinue
    $carapaceExeTime = (Get-Item -LiteralPath 'C:\Users\haris\scoop\apps\carapace\current\carapace.exe').LastWriteTimeUtc
    if (-not $carapaceCacheItem -or $carapaceCacheItem.LastWriteTimeUtc -lt $carapaceExeTime) {
        $null = New-Item -ItemType Directory -Path $carapaceCacheDir -Force
        carapace _carapace | Set-Content -LiteralPath $carapaceCache -Encoding UTF8
    }
    Invoke-Expression (Get-Content -LiteralPath $carapaceCache -Raw)
}

# Oh-My-Posh prompt — half-life theme, initialised the simple way.
$env:POSH_THEME = Join-Path $env:USERPROFILE '.config\oh-my-posh\half-life.omp.json'
if (Test-Path $env:POSH_THEME) {
    oh-my-posh init pwsh --config $env:POSH_THEME | Invoke-Expression
}

# Guarded: this file no longer exists (yasb scripts dir removed) — was throwing CommandNotFoundException on EVERY powershell.exe spawn (startup script, GlazeWM volume/brightness keybinds, etc.)
$yasbThemeShell = "C:\Users\haris\.config\yasb\scripts\yasb-theme-shell.ps1"
if (Test-Path -LiteralPath $yasbThemeShell) { . $yasbThemeShell }

# ============================================================================
# DELAYED SYSTEM ACTIONS — shutdown / reboot / lock / sleep
# Shared duration syntax: plain seconds or e.g., 20s / 10m / 2h.
# Mirrors the PowerShell 7 profile helpers.
# ============================================================================
function lockmyscreen {
    rundll32.exe user32.dll,LockWorkStation
}

function ConvertTo-DelaySeconds {
    param([Parameter(Mandatory)][string]$Time)

    $time = $Time.ToLower()
    if ($time -match '^(\d+)([smh])$') {
        $value = [int]$matches[1]
        switch ($matches[2]) {
            's' { return $value }
            'm' { return $value * 60 }
            'h' { return $value * 3600 }
        }
    }
    if ($time -match '^\d+$') {
        return [int]$time
    }

    throw 'Invalid time format. Use plain seconds or e.g., 20s, 10m, 2h.'
}

function Start-DelayedSystemAction {
    # Lock and sleep have no native "/t seconds" switch, so a detached hidden
    # powershell.exe performs the wait (base64 -EncodedCommand keeps quoting
    # simple). Detached via Start-Process so closing this shell — the way
    # shutdown.exe schedules survive a closed terminal — does not cancel it.
    param(
        [Parameter(Mandatory)][ValidateSet('lock', 'sleep')][string]$Action,
        [Parameter(Mandatory)][int]$Seconds
    )

    if ($Seconds -lt 1) {
        throw 'Delay must be at least 1 second.'
    }

    $inner = if ($Action -eq 'lock') {
        'rundll32.exe user32.dll,LockWorkStation'
    }
    else {
        # SetSuspendState(False, True, False): standby, not hibernate.
        'rundll32.exe powrprof.dll,SetSuspendState 0,1,0'
    }

    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes(
        "Start-Sleep -Seconds $Seconds; $inner"))
    Start-Process -FilePath 'powershell.exe' -WindowStyle Hidden `
        -ArgumentList '-NoProfile', '-EncodedCommand', $encoded
}

function shutdown {
    param([Parameter(Mandatory)][string]$Time)

    shutdown.exe /s /f /t (ConvertTo-DelaySeconds $Time)
}

function reboot {
    param([Parameter(Mandatory)][string]$Time)

    shutdown.exe /r /f /t (ConvertTo-DelaySeconds $Time)
}

function lock {
    param([Parameter(Mandatory)][string]$Time)

    Start-DelayedSystemAction -Action lock -Seconds (ConvertTo-DelaySeconds $Time)
}

function sleep {
    # Overrides the built-in Start-Sleep alias: this `sleep` suspends the
    # system after the delay. Use `Start-Sleep` directly for in-session waits.
    param([Parameter(Mandatory)][string]$Time)

    Start-DelayedSystemAction -Action sleep -Seconds (ConvertTo-DelaySeconds $Time)
}

# Aliases outrank functions in command resolution, so the built-in
# Start-Sleep alias would shadow the function above unless removed.
Remove-Item Alias:sleep -Force -ErrorAction SilentlyContinue
