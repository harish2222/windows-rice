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

# Oh-My-Posh prompt — half-life theme, cached init script.
$env:POSH_THEME = Join-Path $env:USERPROFILE '.config\oh-my-posh\half-life.omp.json'
if (Test-Path $env:POSH_THEME) {
    $poshCacheDir = Join-Path $env:LOCALAPPDATA 'oh-my-posh'
    $poshCache = Join-Path $poshCacheDir 'init-powershell-half-life.ps1'
    if (-not (Test-Path $poshCacheDir)) {
        New-Item -ItemType Directory -Path $poshCacheDir -Force | Out-Null
    }
    if (-not (Test-Path $poshCache) -or (Get-Item $env:POSH_THEME).LastWriteTimeUtc -gt (Get-Item $poshCache).LastWriteTimeUtc) {
        oh-my-posh init powershell --config $env:POSH_THEME | Set-Content -LiteralPath $poshCache -Encoding utf8
    }
    . $poshCache
}

# Guarded: this file no longer exists (yasb scripts dir removed) — was throwing CommandNotFoundException on EVERY powershell.exe spawn (startup script, GlazeWM volume/brightness keybinds, etc.)
$yasbThemeShell = "C:\Users\haris\.config\yasb\scripts\yasb-theme-shell.ps1"
if (Test-Path -LiteralPath $yasbThemeShell) { . $yasbThemeShell }

# ============================================================================
# DELAYED SYSTEM ACTIONS — shutdown / reboot / lock / sleep / signout
# Duration syntax: plain seconds or e.g., 20s / 10m / 2h. Every action also
# accepts `cancel` to abort its pending instance:
#   shutdown cancel / reboot cancel -> shutdown.exe /a (aborts either
#                                      schedule; exit 0 = aborted, 1116 =
#                                      nothing was pending)
#   lock/sleep/signout cancel      -> kills the hidden wait worker below
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
    # Lock, sleep and sign-out have no native "/t seconds" switch, so a
    # detached hidden powershell.exe performs the wait. Detached via
    # Start-Process so closing this shell does not cancel it (matching how
    # shutdown.exe schedules survive). The trailing #hkdevs-delayed-* token
    # is the plaintext tag Stop-DelayedSystemAction hunts for. No quoting is
    # needed: powershell.exe treats everything after -Command as one script
    # and every token below is space-free, so they join to
    #   Start-Sleep -Seconds N ; <action> ; #hkdevs-delayed-<action>
    param(
        [Parameter(Mandatory)][ValidateSet('lock', 'sleep', 'signout')][string]$Action,
        [Parameter(Mandatory)][int]$Seconds
    )

    if ($Seconds -lt 1) {
        throw 'Delay must be at least 1 second.'
    }

    $tokens = switch ($Action) {
        'lock' {
            'Start-Sleep', '-Seconds', "$Seconds", ';',
            'rundll32.exe', 'user32.dll,LockWorkStation', ';', '#hkdevs-delayed-lock'
        }
        'sleep' {
            # SetSuspendState(False, True, False): standby, not hibernate.
            'Start-Sleep', '-Seconds', "$Seconds", ';',
            'rundll32.exe', 'powrprof.dll,SetSuspendState', '0,1,0', ';', '#hkdevs-delayed-sleep'
        }
        'signout' {
            # logoff.exe is the /l logoff of this console session.
            'Start-Sleep', '-Seconds', "$Seconds", ';',
            'logoff.exe', ';', '#hkdevs-delayed-signout'
        }
    }

    Start-Process -FilePath 'powershell.exe' -WindowStyle Hidden `
        -ArgumentList (@('-NoProfile', '-Command') + $tokens)
}

function Stop-DelayedSystemAction {
    # Kills the hidden wait workers carrying the #hkdevs-delayed-<action>
    # tag on their command line; returns how many were killed (0 = none
    # pending). WQL does the whole match, so no $ variables are involved;
    # %hk-delayed-probe9f7% keeps this query's own probe children (which run
    # with a plain `powershell -Command` and would otherwise self-match)
    # from being killed.
    param([ValidateSet('lock', 'sleep', 'signout')][string]$Action)

    $filter = "Name = 'powershell.exe' AND CommandLine LIKE '%hkdevs-delayed-$Action%' " +
        "AND NOT (CommandLine LIKE '%hk-delayed-probe9f7%')"
    $ids = Get-CimInstance -ClassName Win32_Process -Filter $filter -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty ProcessId
    $cancelled = 0
    foreach ($id in $ids) {
        Stop-Process -Id $id -Force -ErrorAction SilentlyContinue
        $cancelled++
    }
    return $cancelled
}

function shutdown {
    # `shutdown <time>` schedules; `shutdown cancel` aborts any pending
    # shutdown or restart schedule.
    param([string]$Operation)

    if ($Operation -match '^(?i)cancel$') {
        shutdown.exe /a 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host 'Pending shutdown cancelled.' -ForegroundColor Green
        }
        else {
            Write-Host 'No pending shutdown to cancel.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Operation) {
        Write-Host 'usage: shutdown [seconds | 20s|10m|2h | cancel]'
        return
    }

    shutdown.exe /s /f /t (ConvertTo-DelaySeconds $Operation)
}

function reboot {
    param([string]$Operation)

    if ($Operation -match '^(?i)cancel$') {
        shutdown.exe /a 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host 'Pending restart cancelled.' -ForegroundColor Green
        }
        else {
            Write-Host 'No pending restart to cancel.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Operation) {
        Write-Host 'usage: reboot [seconds | 20s|10m|2h | cancel]'
        return
    }

    shutdown.exe /r /f /t (ConvertTo-DelaySeconds $Operation)
}

function lock {
    # `lock <time>` schedules; `lock cancel` aborts; a bare `lock` locks now.
    param([string]$Time)

    if ($Time -match '^(?i)cancel$') {
        $cancelled = Stop-DelayedSystemAction -Action lock
        if ($cancelled -gt 0) {
            Write-Host "Cancelled $cancelled delayed lock job(s)." -ForegroundColor Green
        }
        else {
            Write-Host 'No delayed lock pending.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Time) {
        lockmyscreen
        return
    }

    Start-DelayedSystemAction -Action lock -Seconds (ConvertTo-DelaySeconds $Time)
}

function sleep {
    # Overrides the built-in Start-Sleep alias: `sleep <time>` suspends the
    # system after the delay, `sleep cancel` aborts it. Use `Start-Sleep`
    # directly for in-session waits.
    param([string]$Time)

    if ($Time -match '^(?i)cancel$') {
        $cancelled = Stop-DelayedSystemAction -Action sleep
        if ($cancelled -gt 0) {
            Write-Host "Cancelled $cancelled delayed sleep job(s)." -ForegroundColor Green
        }
        else {
            Write-Host 'No delayed sleep pending.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Time) {
        Write-Host 'usage: sleep [seconds | 20s|10m|2h | cancel]'
        return
    }

    Start-DelayedSystemAction -Action sleep -Seconds (ConvertTo-DelaySeconds $Time)
}

function signout {
    # `signout <time>` schedules a delayed sign-out, `signout cancel` aborts
    # it, and a bare `signout` signs off immediately (logoff.exe).
    param([string]$Time)

    if ($Time -match '^(?i)cancel$') {
        $cancelled = Stop-DelayedSystemAction -Action signout
        if ($cancelled -gt 0) {
            Write-Host "Cancelled $cancelled delayed sign-out job(s)." -ForegroundColor Green
        }
        else {
            Write-Host 'No delayed sign-out pending.' -ForegroundColor Yellow
        }
        return
    }
    if ($null -eq $Time) {
        logoff.exe
        return
    }

    Start-DelayedSystemAction -Action signout -Seconds (ConvertTo-DelaySeconds $Time)
}

# Aliases outrank functions in command resolution, so the built-in
# Start-Sleep alias would shadow the function above unless removed.
Remove-Item Alias:sleep -Force -ErrorAction SilentlyContinue

# Memory-friendly aliases: logout == signout, restart == reboot (both accept
# the same delay or `cancel`, so e.g. `logout cancel` and `restart cancel`
# behave exactly like their canonical verbs).
Set-Alias logout signout
Set-Alias restart reboot

function Show-Help {
    # Lists everything this profile adds (PowerShell resolves names
    # case-insensitively, so `show-help` reaches this function too).
@'
PowerShell 5.1 Profile Help
===========================

Delayed system actions (duration: plain seconds or 20s / 10m / 2h):
  shutdown <time>   Schedule forceful shutdown.
  reboot <time>     Schedule a restart.
  lock <time>       Lock the workstation after the delay (bare: lock now).
  sleep <time>      Suspend the system after the delay.
  signout <time>    Sign out after the delay (bare: signs off now).
  logout/restart    Same as signout/reboot (alias keywords for memory).
  <cmd> cancel      Abort: shutdown/reboot cancel -> shutdown /a; the
                    lock/sleep/signout pair kills its hidden wait worker.

Other:
  lockmyscreen      Lock the screen immediately.
  show-help         Show this help.
'@ | Write-Host
}
