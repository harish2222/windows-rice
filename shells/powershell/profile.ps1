# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: PowerShell 7 Common Profile Configuration
# ============================================================================
# All-hosts profile (every PowerShell host).
# Startup critical path only: nothing here may block for more than a few ms.

# ─── Oh-My-Posh — only for hosts whose current-host profile does not init it ──
# (ConsoleHost / Otty / Windows Terminal load the current-host profile, which
# initializes the same Half-Life theme; skip the duplicate work.)
if ($Host.Name -ne 'ConsoleHost') {
    $ompTheme = Join-Path $env:USERPROFILE '.config\oh-my-posh\half-life.omp.json'
    if ((Test-Path $ompTheme) -and (Get-Command oh-my-posh -ErrorAction SilentlyContinue)) {
        oh-my-posh init pwsh --config $ompTheme | Invoke-Expression
    }
}

# Fast existence probes: Get-Command walks every PATH directory (~30-80 ms each).
# Known install locations are checked first; Get-Command stays as fallback.
function Test-FastCommand {
    param([string]$Name)
    foreach ($dir in @((Join-Path $HOME 'scoop\shims'), (Join-Path $HOME '.local\bin'), (Join-Path $env:LOCALAPPDATA 'Programs'))) {
        foreach ($ext in '.exe', '.cmd', '.bat', '.ps1') {
            if (Test-Path -LiteralPath (Join-Path $dir ($Name + $ext))) { return $true }
        }
    }
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

# ─── zoxide cd replacement (cached init with automatic directory tracking) ───
if (Test-FastCommand zoxide) {
    $env:_ZO_ECHO = "1"
    $env:_ZO_RESOLVE_SYMLINKS = "1"
    $env:_ZO_FZF_OPTS = "--height 15 --min-height 10 --layout reverse --border rounded --info inline --prompt 'Zoxide❯ '"

    $zoxideCacheDir = Join-Path $env:LOCALAPPDATA 'zoxide'
    $zoxideCache = Join-Path $zoxideCacheDir 'init-pwsh.ps1'
    $zoxideCmd = Get-Command zoxide -ErrorAction SilentlyContinue
    $zoxideExe = if ($zoxideCmd) { $zoxideCmd.Source } else { $null }

    if (-not (Test-Path $zoxideCacheDir)) {
        New-Item -ItemType Directory -Path $zoxideCacheDir -Force | Out-Null
    }
    if ($zoxideExe -and ((-not (Test-Path $zoxideCache)) -or ((Get-Item $zoxideExe).LastWriteTimeUtc -gt (Get-Item $zoxideCache).LastWriteTimeUtc))) {
        & $zoxideExe init powershell --cmd cd | Set-Content -LiteralPath $zoxideCache -Encoding UTF8
    }
    if (Test-Path $zoxideCache) {
        . $zoxideCache
        Set-Alias -Name z -Value __zoxide_z -Option AllScope -Scope Global -Force
        Set-Alias -Name zi -Value __zoxide_zi -Option AllScope -Scope Global -Force
    }
}

# ─── Bat man-pages ────────────────────────────────────────────────────────────
if (Test-FastCommand bat) {
    $env:MANPAGER = "sh -c 'col -bx | bat -l man -p'"
}

# ─── fzf + PSFzf — lazily imported ────────────────────────────────────────────
# PSFzf's Import-Module + Set-PsFzfOption cost >1s at startup. It is now loaded
# the first time Ctrl+R / Ctrl+T / Ctrl+Space / Alt+C is pressed (or by running Enable-PSFzf).
if (Test-FastCommand fzf) {
    $script:PSFzfLoaded = $false

    $env:FZF_DEFAULT_OPTS = " --height 15 --min-height 10 --layout reverse --border rounded --info inline" +
        " --color=bg+:#363a4f,bg:#1e1e2e,spinner:#f5e0dc,hl:#f38ba8" +
        " --color=fg:#cdd6f4,header:#f38ba8,info:#cba6f7,pointer:#f5e0dc" +
        " --color=marker:#b4befe,fg+:#cdd6f4,prompt:#cba6f7,hl+:#f38ba8" +
        " --color=selected-bg:#45475a,border:#89b4fa"

    function global:Invoke-FuzzyEnv {
        $selected = Get-ChildItem env: | ForEach-Object { "$($_.Name)=$($_.Value)" } | fzf --prompt 'Env❯ ' --height 15 --min-height 10 --layout reverse --border rounded
        if ($selected) {
            $varName = ($selected -split '=')[0]
            if (Get-Module -Name PSReadLine -ErrorAction SilentlyContinue) {
                [Microsoft.PowerShell.PSConsoleReadLine]::Insert("`$env:$varName")
            } else {
                "`$env:$varName"
            }
        }
    }

    function global:Invoke-FuzzyGitLog {
        $hash = git log --oneline --color=always 2>$null | fzf --ansi --prompt 'Commit❯ ' --height 15 --min-height 10 --layout reverse --border rounded --preview 'git show --color=always {1}'
        if ($hash) {
            $cleanHash = ($hash -split '\s+')[0]
            if (Get-Module -Name PSReadLine -ErrorAction SilentlyContinue) {
                [Microsoft.PowerShell.PSConsoleReadLine]::Insert($cleanHash)
            } else {
                $cleanHash
            }
        }
    }

    function Enable-PSFzf {
        if ($script:PSFzfLoaded) { return }
        $script:PSFzfLoaded = $true
        Import-Module PSFzf -ErrorAction SilentlyContinue
        if (Get-Command Set-PsFzfOption -ErrorAction SilentlyContinue) {
            $fzfOptions = @{
                PSReadlineChordProvider = 'Ctrl+t'
                PSReadlineChordReverseHistory = 'Ctrl+r'
                PSReadlineChordSetLocation = 'Alt+c'
                TabExpansion = $true
                GitKeyBindings = $true
                EnableAliasFuzzyEdit = $true
                EnableAliasFuzzyHistory = $true
                EnableAliasFuzzyKillProcess = $true
                EnableAliasFuzzyScoop = $true
                EnableAliasFuzzyGitStatus = $true
                EnableAliasFuzzySetLocation = $true
                EnableFd = $true
            }
            Set-PsFzfOption @fzfOptions
            Set-Alias -Name fcd -Value Invoke-FuzzySetLocation -Scope Global -Force -ErrorAction SilentlyContinue
            Set-Alias -Name fhistory -Value Invoke-FuzzyHistory -Scope Global -Force -ErrorAction SilentlyContinue
            Set-Alias -Name fgb -Value Invoke-PsFzfGitBranches -Scope Global -Force -ErrorAction SilentlyContinue
            Set-Alias -Name fgh -Value Invoke-PsFzfGitHashes -Scope Global -Force -ErrorAction SilentlyContinue
            Set-Alias -Name fenv -Value Invoke-FuzzyEnv -Scope Global -Force -ErrorAction SilentlyContinue
            Set-Alias -Name fgl -Value Invoke-FuzzyGitLog -Scope Global -Force -ErrorAction SilentlyContinue
        }
    }

    if ($Host.Name -eq 'ConsoleHost' -and (Get-Module -Name PSReadLine)) {
        # First press loads PSFzf (which rebinds these); built-in search is used
        # for the very first press so the key is never dead.
        Set-PSReadLineKeyHandler -Chord 'Ctrl+r' -ScriptBlock {
            Enable-PSFzf
            [Microsoft.PowerShell.PSConsoleReadLine]::ReverseSearchHistory()
        }
        Set-PSReadLineKeyHandler -Chord 'Ctrl+t' -ScriptBlock {
            Enable-PSFzf
        }
        Set-PSReadLineKeyHandler -Chord 'Alt+c' -ScriptBlock {
            Enable-PSFzf
        }
        Set-PSReadLineKeyHandler -Chord 'Ctrl+Spacebar' -ScriptBlock {
            Enable-PSFzf
            if (Get-Command Invoke-FzfTabCompletion -ErrorAction SilentlyContinue) {
                Invoke-FzfTabCompletion
            } else {
                [Microsoft.PowerShell.PSConsoleReadLine]::MenuComplete()
            }
        }
    }
}

function window-rice {
    [CmdletBinding()]
    param(
        [Alias('v')][switch]$Verify,
        [switch]$LinkAll,
        [Alias('x')][switch]$Unlink,
        [switch]$UnlinkAll,
        [switch]$UninstallAll,
        [switch]$InstallMissing,
        [Alias('r')][switch]$Reload,
        [Alias('c')][switch]$Update,
        [Alias('w')][switch]$Wallpapers,
        [string]$WallpaperDir,
        [switch]$UninstallWallpapers,
        [Alias('b')][switch]$Bootstrap
    )
    $targetExe = Join-Path $HOME ".config\installer\target\release\rice-installer.exe"
    $installPs1 = Join-Path $HOME ".config\install.ps1"
    $cargoToml = Join-Path $HOME ".config\installer\Cargo.toml"

    if ($Reload) {
        if (Get-Command komorebic -ErrorAction SilentlyContinue) {
            & komorebic replace-configuration "$HOME\.config\komorebi\komorebi.json" 2>$null
        }
        if (Get-Command glazewm -ErrorAction SilentlyContinue) {
            & glazewm command wm-reload-config 2>$null
        }
        $psExe = if (Get-Command pwsh -ErrorAction SilentlyContinue) { "pwsh" } else { "powershell" }
        $yasbScript = Join-Path $HOME ".config\yasb\tools\setup\yasb-setup.ps1"
        if (Test-Path $yasbScript) {
            & $psExe -NoProfile -ExecutionPolicy Bypass -File $yasbScript
        }
        Write-Host "Windows Rice environment reloaded!" -ForegroundColor Green
        return
    }

    if (-not (Test-Path $targetExe) -and (Test-Path $cargoToml) -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
        & cargo build --release --manifest-path $cargoToml --quiet
    }

    $passArgs = @()
    if ($Verify) { $passArgs += "--verify" }
    if ($LinkAll) { $passArgs += "--link-all" }
    if ($Unlink) { $passArgs += "--unlink" }
    if ($UnlinkAll) { $passArgs += "--unlink-all" }
    if ($UninstallAll) {
        Write-Host "FULL REVOKE requested: every rice-installed config & wallpaper entry will be removed (binaries stay)." -ForegroundColor Red
        $passArgs += "--uninstall-all"
    }
    if ($InstallMissing) { $passArgs += "--install-missing" }
    if ($Update) { $passArgs += "--update" }
    if ($Wallpapers -or $WallpaperDir) {
        $passArgs += "--wallpapers"
        if ($WallpaperDir) { $passArgs += $WallpaperDir }
    }
    if ($UninstallWallpapers) { $passArgs += "--uninstall-wallpapers" }
    if ($Bootstrap) { $passArgs += "--bootstrap" }

    if (Test-Path $targetExe) {
        & $targetExe @passArgs
    } elseif (Test-Path $installPs1) {
        $psExe = if (Get-Command pwsh -ErrorAction SilentlyContinue) { "pwsh" } else { "powershell" }
        & $psExe -NoProfile -File $installPs1 @passArgs
    }
}
Set-Alias -Name rice -Value window-rice -Force
