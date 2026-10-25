<#
.SYNOPSIS
    Windows Rice Bootstrap & Interactive Installer Launcher
.DESCRIPTION
    Launches the Rust-powered Ratatui TUI installer (rice-installer), or provides
    automated bootstrapping for package managers (Scoop, Winget, Chocolatey, Cargo).
.EXAMPLE
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
$ScriptRoot = $PSScriptRoot

if ($Help) {
    Write-Host @"
WINDOWS RICE INSTALLER LAUNCHER
================================
Usage: .\install.ps1 [OPTIONS]

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

# 1. Check if cargo is available
$hasCargo = (Get-Command cargo -ErrorAction SilentlyContinue) -ne $null
$installerDir = Join-Path $ScriptRoot "installer"
$manifestPath = Join-Path $installerDir "Cargo.toml"

# Pass-through args
$extraArgs = @()
if ($Verify) { $extraArgs += "--verify" }
if ($LinkAll) { $extraArgs += "--link-all" }
if ($InstallMissing) { $extraArgs += "--install-missing" }
if ($Bootstrap) { $extraArgs += "--bootstrap" }

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

# 2. Check pre-built binary
$prebuiltExe = Join-Path $installerDir "target\release\rice-installer.exe"
if (Test-Path $prebuiltExe) {
    & $prebuiltExe $extraArgs
    exit $LASTEXITCODE
}

# 3. Fallback: Cargo is missing; offer to bootstrap or install
Write-Host "[Windows Rice] Rust compiler (cargo) is not detected." -ForegroundColor Yellow
Write-Host "Would you like to install Scoop & Rust, or run quick native linking?" -ForegroundColor Cyan
Write-Host "  [1] Bootstrap Scoop and Rust compiler (Recommended for full TUI)"
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
        # Direct fallback linking logic
        $userProfile = $env:USERPROFILE
        $destMap = @{
            "glazewm"          = "$userProfile\.glzr\glazewm"
            "komorebi"         = "$userProfile\.config\komorebi"
            "yasb"             = "$userProfile\.config\yasb"
            "otty"             = "$userProfile\.config\otty"
            "carapace"         = "$userProfile\.config\carapace"
            "carapace\specs"   = "$env:APPDATA\carapace\specs"
            "shells\powershell"= "$userProfile\OneDrive\Documents\PowerShell"
            "shells\nushell"   = "$userProfile\.config\nushell"
            "shells\xonsh"     = "$userProfile\.config\xonsh"
        }

        foreach ($src in $destMap.Keys) {
            $srcPath = Join-Path $ScriptRoot $src
            $dstPath = $destMap[$src]
            if (Test-Path $srcPath) {
                $dstParent = Split-Path $dstPath -Parent
                if (-not (Test-Path $dstParent)) { New-Item -ItemType Directory -Force -Path $dstParent | Out-Null }
                try {
                    New-Item -ItemType SymbolicLink -Path $dstPath -Target $srcPath -Force -ErrorAction Stop | Out-Null
                    Write-Host "  [SYMLINK] $src -> $dstPath" -ForegroundColor Green
                } catch {
                    Copy-Item -Path $srcPath -Destination $dstPath -Recurse -Force
                    Write-Host "  [COPIED]  $src -> $dstPath" -ForegroundColor Cyan
                }
            }
        }
        Write-Host "[Windows Rice] Fallback linking complete!" -ForegroundColor Green
    }
    default {
        Write-Host "Exiting."
        exit 0
    }
}
