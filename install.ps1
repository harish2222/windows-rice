<#
.SYNOPSIS
    Windows Rice Bootstrap & Interactive Installer Launcher
.DESCRIPTION
    Supports both local execution and single-command remote execution via iwr / irm:
        irm https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex
        iwr -useb https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex

    Launches the Rust-powered Ratatui TUI installer (rice-installer), or provides
    automated bootstrapping for package managers (Scoop, Winget, Chocolatey, Cargo).
.EXAMPLE
    irm https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex
    .\install.ps1
    .\install.ps1 -Verify
    .\install.ps1 -LinkAll
    .\install.ps1 -InstallMissing
#>

[CmdletBinding()]
param(
    [switch]$Verify,
    [switch]$LinkAll,
    [switch]$InstallMissing,
    [switch]$Bootstrap,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host @"
WINDOWS RICE INSTALLER LAUNCHER
================================
Remote Web One-Liner (iwr / irm):
    irm https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex
    iwr -useb https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex

Local Usage:
    .\install.ps1 [OPTIONS]

Options:
    -Verify          Run comprehensive system health check and exit
    -LinkAll         Link all configuration files without prompt
    -InstallMissing  Install missing binaries and link configs
    -Bootstrap       Bootstrap Scoop and Chocolatey package managers
    -Help            Show this help text

Running without options launches the interactive Rust Ratatui TUI.
"@
    exit 0
}

# -----------------------------------------------------------------------------
# 0. Remote Execution Detection (iwr / irm | iex)
# -----------------------------------------------------------------------------
$targetRepoDir = Join-Path $env:USERPROFILE ".config"
$isRemote = [string]::IsNullOrEmpty($PSScriptRoot) -or (-not (Test-Path (Join-Path $PSScriptRoot "installer\Cargo.toml")))

if ($isRemote) {
    Write-Host "[Windows Rice] Remote execution detected (iwr / irm)." -ForegroundColor Cyan
    $repoUrl = "https://github.com/harish2222/windows-rice.git"

    if (Test-Path (Join-Path $targetRepoDir ".git")) {
        Write-Host "[Windows Rice] Repository already exists at $targetRepoDir." -ForegroundColor Green
        if (Get-Command git -ErrorAction SilentlyContinue) {
            Write-Host "[Windows Rice] Pulling latest changes..." -ForegroundColor DarkGray
            try { git -C $targetRepoDir pull --quiet origin main } catch {}
        }
    } else {
        Write-Host "[Windows Rice] Setting up repository at $targetRepoDir..." -ForegroundColor Cyan
        if (Get-Command git -ErrorAction SilentlyContinue) {
            git clone $repoUrl $targetRepoDir
        } else {
            Write-Host "[Windows Rice] Git not found. Downloading repository archive..." -ForegroundColor Yellow
            $tempZip = Join-Path $env:TEMP "windows-rice.zip"
            $tempExtract = Join-Path $env:TEMP "windows-rice-extract"
            if (Test-Path $tempZip) { Remove-Item $tempZip -Force }
            if (Test-Path $tempExtract) { Remove-Item $tempExtract -Recurse -Force }

            Invoke-RestMethod -Uri "https://github.com/harish2222/windows-rice/archive/refs/heads/main.zip" -OutFile $tempZip
            Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force

            $extractedFolder = Join-Path $tempExtract "windows-rice-main"
            if (-not (Test-Path $targetRepoDir)) {
                New-Item -ItemType Directory -Force -Path $targetRepoDir | Out-Null
            }
            Copy-Item -Path "$extractedFolder\*" -Destination $targetRepoDir -Recurse -Force
            Remove-Item $tempZip -Force -ErrorAction SilentlyContinue
            Remove-Item $tempExtract -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    # Forward execution to local script inside targetRepoDir
    $localScript = Join-Path $targetRepoDir "install.ps1"
    & $localScript @PSBoundParameters
    exit $LASTEXITCODE
}

# -----------------------------------------------------------------------------
# 1. Local Execution inside Repository
# -----------------------------------------------------------------------------
$ScriptRoot = $PSScriptRoot
$installerDir = Join-Path $ScriptRoot "installer"
$manifestPath = Join-Path $installerDir "Cargo.toml"

# Pass-through args for the rust binary
$extraArgs = @()
if ($Verify) { $extraArgs += "--verify" }
if ($LinkAll) { $extraArgs += "--link-all" }
if ($InstallMissing) { $extraArgs += "--install-missing" }
if ($Bootstrap) { $extraArgs += "--bootstrap" }

$hasCargo = (Get-Command cargo -ErrorAction SilentlyContinue) -ne $null

# 2. Check if Cargo is available to compile/run TUI
if ($hasCargo -and (Test-Path $manifestPath)) {
    Write-Host "[Windows Rice] Launching Rust TUI installer..." -ForegroundColor Cyan
    Push-Location $installerDir
    try {
        & cargo run --release -- $extraArgs
    } finally {
        Pop-Location
    }
    exit $LASTEXITCODE
}

# 3. Check pre-built binary
$prebuiltExe = Join-Path $installerDir "target\release\rice-installer.exe"
if (Test-Path $prebuiltExe) {
    & $prebuiltExe $extraArgs
    exit $LASTEXITCODE
}

# 4. Fallback: Cargo is missing; offer to bootstrap or install
Write-Host "[Windows Rice] Rust compiler (cargo) is not detected." -ForegroundColor Yellow
Write-Host "Would you like to install Scoop & Rust, or run quick native linking?" -ForegroundColor Cyan
Write-Host "  [1] Bootstrap Scoop and Rust compiler (Recommended for full Ratatui TUI)"
Write-Host "  [2] Run native PowerShell config linking directly"
Write-Host "  [3] Exit"

$choice = Read-Host "Select option [1/2/3]"

switch ($choice) {
    "1" {
        Write-Host "Bootstrapping Scoop..." -ForegroundColor Green
        Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
        Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
        scoop install rust
        & cargo run --release --manifest-path $manifestPath -- $extraArgs
    }
    "2" {
        Write-Host "Running native config linking..." -ForegroundColor Green
        $userProfile = $env:USERPROFILE
        $appData = $env:APPDATA

        $pwshDest = if (Test-Path "$userProfile\OneDrive\Documents") {
            "$userProfile\OneDrive\Documents\PowerShell"
        } else {
            "$userProfile\Documents\PowerShell"
        }

        $destMap = @{
            "glazewm"                           = "$userProfile\.glzr\glazewm"
            "komorebi"                          = "$userProfile\.config\komorebi"
            "yasb"                              = "$userProfile\.config\yasb"
            "hotkeys-overlay"                   = "$userProfile\.config\hotkeys-overlay"
            "windows-terminal\settings.json"    = "$env:LOCALAPPDATA\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json"
            "otty"                              = "$userProfile\.config\otty"
            "wezterm"                           = "$userProfile\.config\wezterm"
            "wezterm\wezterm.lua"               = "$userProfile\.wezterm.lua"
            "alacritty"                         = "$userProfile\.config\alacritty"
            "alacritty\alacritty.toml"          = "$appData\alacritty\alacritty.toml"
            "rio"                               = "$userProfile\.config\rio"
            "kew"                               = "$appData\kew"
            "cliamp"                            = "$userProfile\.config\cliamp"
            "cava"                              = "$userProfile\.config\cava"
            "mpd"                               = "$userProfile\.config\mpd"
            "ncmpc"                             = "$userProfile\.ncmpc"
            "fastfetch"                         = "$userProfile\.config\fastfetch"
            "carapace"                          = "$userProfile\.config\carapace"
            "carapace\specs"                    = "$appData\carapace\specs"
            "shells\powershell"                 = $pwshDest
            "shells\nushell"                    = "$appData\nushell"
            "shells\xonsh"                      = "$appData\xonsh"
        }

        foreach ($src in $destMap.Keys) {
            $srcPath = Join-Path $ScriptRoot $src
            $dstPath = $destMap[$src]
            if (Test-Path $srcPath) {
                # Skip if src and dst point to the same directory
                if ((Resolve-Path $srcPath -ErrorAction SilentlyContinue).Path -eq (Resolve-Path $dstPath -ErrorAction SilentlyContinue).Path) {
                    Write-Host "  [IN-PLACE] $src -> already in $dstPath" -ForegroundColor DarkGray
                    continue
                }
                $dstParent = Split-Path $dstPath -Parent
                if (-not (Test-Path $dstParent)) { New-Item -ItemType Directory -Force -Path $dstParent | Out-Null }
                try {
                    New-Item -ItemType SymbolicLink -Path $dstPath -Target $srcPath -Force -ErrorAction Stop | Out-Null
                    Write-Host "  [SYMLINK]  $src -> $dstPath" -ForegroundColor Green
                } catch {
                    Copy-Item -Path $srcPath -Destination $dstPath -Recurse -Force
                    Write-Host "  [COPIED]   $src -> $dstPath" -ForegroundColor Cyan
                }
            }
        }

        # Also copy .xonshrc to home if present
        $xonshrc = Join-Path $ScriptRoot "shells\xonsh\.xonshrc"
        if (Test-Path $xonshrc) {
            Copy-Item $xonshrc "$userProfile\.xonshrc" -Force
            Write-Host "  [COPIED]   shells\xonsh\.xonshrc -> $userProfile\.xonshrc" -ForegroundColor Cyan
        }

        Write-Host "[Windows Rice] Fallback linking complete!" -ForegroundColor Green
    }
    default {
        Write-Host "Exiting."
        exit 0
    }
}
