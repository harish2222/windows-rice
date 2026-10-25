# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: WINDOW-RICE Turnkey Bootstrap & Installer Launcher
# ============================================================================
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
    [string]$Tag = "v1.0.0",
    [switch]$Verify,
    [switch]$LinkAll,
    [switch]$Unlink,
    [switch]$UnlinkAll,
    [switch]$Install,
    [switch]$InstallMissing,
    [switch]$Bootstrap,
    [switch]$Reload,
    [switch]$Update,
    [switch]$Wallpapers,
    [switch]$UninstallWallpapers,
    [string]$WallpaperDir,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# Prevent Internet Explorer DOM parser errors on Windows PowerShell 5.1
$PSDefaultParameterValues['Invoke-WebRequest:UseBasicParsing'] = $true
$PSDefaultParameterValues['Invoke-RestMethod:UseBasicParsing'] = $true

# Force TLS 1.2 and TLS 1.3 protocol for GitHub and HTTPS connectivity in PowerShell 5.1
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls13
} catch {}

# Resolve PowerShell executable: pwsh (PS 7+) if present, falling back to powershell (PS 5.1)
$psExe = if (Get-Command pwsh -ErrorAction SilentlyContinue) { "pwsh" } else { "powershell" }

if ($Help) {
    Write-Host @"
WINDOW-RICE INSTALLER LAUNCHER
==============================
Remote Web One-Liner (iwr / irm):
    irm https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex
    iwr -useb https://raw.githubusercontent.com/harish2222/windows-rice/main/install.ps1 | iex

Local Usage:
    .\install.ps1 [OPTIONS]

Options:
    -Reload               Reload Komorebi WM, GlazeWM hotkeys, and YASB status bar
    -Update               Check repository for upstream updates
    -Wallpapers           Install 8K wallpapers to default (Pictures\Wallpapers) or -WallpaperDir
    -UninstallWallpapers  Remove 8K wallpapers from Pictures\Wallpapers
    -Verify               Run comprehensive system health check and exit
    -LinkAll              Link all configuration files without prompt (backs up user configs)
    -Unlink               Unlink selected/installed configurations cleanly and restore backups
    -UnlinkAll            Unlink all configurations cleanly without trace and restore backups
    -Install              Install missing binaries and link configs (with automated tests)
    -InstallMissing       Install missing binaries and link configs (alias for -Install)
    -Bootstrap            Bootstrap Scoop, Chocolatey, UV, and Vcpkg
    -Help                 Show this help text

Running without options launches the interactive Rust Ratatui TUI (WINDOW-RICE installer).
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
    $tempInstaller = Join-Path $env:TEMP "rice-installer.exe"
    $releaseExeUrl = "https://github.com/harish2222/windows-rice/releases/download/$Tag/rice-installer.exe"
    $latestExeUrl = "https://github.com/harish2222/windows-rice/releases/latest/download/rice-installer.exe"

    # Step 1: Ensure Scoop is installed
    $scoopShim = Join-Path $env:USERPROFILE "scoop\shims"
    $env:PATH = "$scoopShim;$env:PATH"
    if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
        Write-Host "[Windows Rice] Step 1/4: Installing Scoop package manager..." -ForegroundColor Cyan
        try {
            Invoke-RestMethod -Uri https://get.scoop.sh -UseBasicParsing | Invoke-Expression
        } catch {
            try {
                Invoke-Expression (New-Object System.Net.WebClient).DownloadString('https://get.scoop.sh')
            } catch {
                Write-Host "[Windows Rice] Scoop setup notice: $_" -ForegroundColor Yellow
            }
        }
        $env:PATH = "$scoopShim;$env:PATH"
    }

    # Step 2: Ensure Git is installed via Scoop
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        Write-Host "[Windows Rice] Step 2/4: Installing Git via Scoop..." -ForegroundColor Cyan
        if (Get-Command scoop -ErrorAction SilentlyContinue) {
            & scoop install git
        }
        $gitCmd = Join-Path $env:USERPROFILE "scoop\apps\git\current\cmd"
        $gitBin = Join-Path $env:USERPROFILE "scoop\apps\git\current\bin"
        $env:PATH = "$scoopShim;$gitCmd;$gitBin;$env:PATH"
    }

    # Step 3: Clone repository via git clone --branch <tag_name> --depth 1 <repository_url>
    Write-Host "[Windows Rice] Step 3/4: Setting up repository via git clone (branch/tag: $Tag)..." -ForegroundColor Cyan
    $cloned = $false
    if (Get-Command git -ErrorAction SilentlyContinue) {
        if (-not (Test-Path $targetRepoDir)) {
            Write-Host "[Windows Rice] Running: git clone --branch $Tag --depth 1 $repoUrl $targetRepoDir" -ForegroundColor DarkGray
            git clone --branch $Tag --depth 1 $repoUrl $targetRepoDir
            if ($LASTEXITCODE -eq 0 -and (Test-Path (Join-Path $targetRepoDir "install.ps1"))) {
                $cloned = $true
            }
        } else {
            # Target directory already exists (e.g. existing ~/.config in Windows Sandbox or local user)
            if (Test-Path (Join-Path $targetRepoDir ".git")) {
                Write-Host "[Windows Rice] Repository already exists at $targetRepoDir. Updating..." -ForegroundColor DarkGray
                try {
                    git -C $targetRepoDir fetch --depth 1 origin $Tag
                    git -C $targetRepoDir checkout $Tag
                } catch {
                    try { git -C $targetRepoDir pull --quiet origin main } catch {}
                }
                $cloned = $true
            } else {
                # Target exists but is not a git repo: clone into temp and merge into targetRepoDir
                $tempClone = Join-Path $env:TEMP "windows-rice-clone-$([Guid]::NewGuid().ToString('N'))"
                if (Test-Path $tempClone) { Remove-Item $tempClone -Recurse -Force -ErrorAction SilentlyContinue }
                Write-Host "[Windows Rice] Running: git clone --branch $Tag --depth 1 $repoUrl $tempClone" -ForegroundColor DarkGray
                git clone --branch $Tag --depth 1 $repoUrl $tempClone
                if ($LASTEXITCODE -eq 0 -and (Test-Path $tempClone)) {
                    Copy-Item -Path "$tempClone\*" -Destination $targetRepoDir -Recurse -Force
                    if (Test-Path (Join-Path $tempClone ".git")) {
                        Copy-Item -Path (Join-Path $tempClone ".git") -Destination (Join-Path $targetRepoDir ".git") -Recurse -Force
                    }
                    Remove-Item $tempClone -Recurse -Force -ErrorAction SilentlyContinue
                    $cloned = $true
                }
            }
        }
    }

    # Fallback archive download if git clone encountered network issue
    if (-not $cloned -and (-not (Test-Path (Join-Path $targetRepoDir "install.ps1")))) {
        Write-Host "[Windows Rice] Downloading repository archive from GitHub..." -ForegroundColor Yellow
        $tempZip = Join-Path $env:TEMP "windows-rice.zip"
        $tempExtract = Join-Path $env:TEMP "windows-rice-extract"
        if (Test-Path $tempZip) { Remove-Item $tempZip -Force -ErrorAction SilentlyContinue }
        if (Test-Path $tempExtract) { Remove-Item $tempExtract -Recurse -Force -ErrorAction SilentlyContinue }

        $zipUrl = "https://github.com/harish2222/windows-rice/archive/refs/tags/$Tag.zip"
        try {
            Invoke-WebRequest -Uri $zipUrl -OutFile $tempZip -UseBasicParsing -ErrorAction Stop
        } catch {
            try {
                $zipUrl = "https://github.com/harish2222/windows-rice/archive/refs/heads/main.zip"
                Invoke-WebRequest -Uri $zipUrl -OutFile $tempZip -UseBasicParsing -ErrorAction Stop
            } catch {}
        }

        if (Test-Path $tempZip) {
            Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force
            $extractedFolder = Get-ChildItem -Path $tempExtract -Directory | Select-Object -First 1
            if (-not (Test-Path $targetRepoDir)) {
                New-Item -ItemType Directory -Force -Path $targetRepoDir | Out-Null
            }
            if ($extractedFolder) {
                Copy-Item -Path "$($extractedFolder.FullName)\*" -Destination $targetRepoDir -Recurse -Force
            }
            Remove-Item $tempZip -Force -ErrorAction SilentlyContinue
            Remove-Item $tempExtract -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    # Step 4: Ensure pre-compiled binary is downloaded and ready in TEMP
    if (-not (Test-Path $tempInstaller) -or ((Get-Item $tempInstaller).Length -lt 500000)) {
        Write-Host "[Windows Rice] Step 4/4: Downloading pre-compiled installer binary ($Tag)..." -ForegroundColor Cyan
        try {
            Invoke-WebRequest -Uri $releaseExeUrl -OutFile $tempInstaller -UseBasicParsing -ErrorAction Stop
        } catch {
            try {
                Invoke-WebRequest -Uri $latestExeUrl -OutFile $tempInstaller -UseBasicParsing -ErrorAction Stop
            } catch {
                try {
                    $webClient = New-Object System.Net.WebClient
                    $webClient.DownloadFile($latestExeUrl, $tempInstaller)
                } catch {
                    if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
                        try { & curl.exe -sSL $latestExeUrl -o $tempInstaller } catch {}
                    }
                }
            }
        }
    }

    # Step 5: Launch TUI installer immediately
    if (Test-Path $tempInstaller -and ((Get-Item $tempInstaller).Length -ge 500000)) {
        # Copy to repository as well if present
        $repoBuilt = Join-Path $targetRepoDir "installer\target\release\rice-installer.exe"
        $repoBuiltDir = Split-Path $repoBuilt -Parent
        if (-not (Test-Path $repoBuiltDir)) { New-Item -ItemType Directory -Force -Path $repoBuiltDir | Out-Null }
        Copy-Item $tempInstaller $repoBuilt -Force -ErrorAction SilentlyContinue

        $extraArgs = @()
        if ($Verify) { $extraArgs += "--verify" }
        if ($LinkAll) { $extraArgs += "--link-all" }
        if ($Unlink) { $extraArgs += "--unlink" }
        if ($UnlinkAll) { $extraArgs += "--unlink-all" }
        if ($InstallMissing -or $Install) { $extraArgs += "--install-missing" }
        if ($Bootstrap) { $extraArgs += "--bootstrap" }
        if ($Reload) { $extraArgs += "--reload" }
        if ($Update) { $extraArgs += "--update" }
        if ($UninstallWallpapers) { $extraArgs += "--uninstall-wallpapers" }
        if ($Wallpapers) {
            $extraArgs += "--wallpapers"
            if ($WallpaperDir) { $extraArgs += $WallpaperDir }
        }

        Write-Host "[Windows Rice] Launching WINDOW-RICE Ratatui TUI installer..." -ForegroundColor Green
        & $tempInstaller $extraArgs
        exit $LASTEXITCODE
    }

    # Fallback to local script inside targetRepoDir
    $localScript = Join-Path $targetRepoDir "install.ps1"
    if (Test-Path $localScript) {
        & $psExe -NoProfile -ExecutionPolicy Bypass -File $localScript @PSBoundParameters
        exit $LASTEXITCODE
    } else {
        Write-Host "[Windows Rice] Error: Failed to locate $localScript after repository setup." -ForegroundColor Red
        exit 1
    }
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
if ($Unlink) { $extraArgs += "--unlink" }
if ($UnlinkAll) { $extraArgs += "--unlink-all" }
if ($InstallMissing -or $Install) { $extraArgs += "--install-missing" }
if ($Bootstrap) { $extraArgs += "--bootstrap" }
if ($Reload) { $extraArgs += "--reload" }
if ($Update) { $extraArgs += "--update" }
if ($UninstallWallpapers) { $extraArgs += "--uninstall-wallpapers" }
if ($Wallpapers) {
    $extraArgs += "--wallpapers"
    if ($WallpaperDir) { $extraArgs += $WallpaperDir }
}

$tempInstaller = Join-Path $env:TEMP "rice-installer.exe"
$builtExe = Join-Path $installerDir "target\release\rice-installer.exe"

# 2. If already built in repository, deploy to temp
if (Test-Path $builtExe) {
    Copy-Item $builtExe $tempInstaller -Force -ErrorAction SilentlyContinue
}

# 3. Check if pre-compiled release binary is available from GitHub
if (-not (Test-Path $tempInstaller)) {
    Write-Host "[Windows Rice] Checking for pre-compiled binary from GitHub releases..." -ForegroundColor DarkGray
    $releaseExeUrl = "https://github.com/harish2222/windows-rice/releases/latest/download/rice-installer.exe"
    try {
        Invoke-WebRequest -Uri $releaseExeUrl -OutFile $tempInstaller -UseBasicParsing -ErrorAction Stop
    } catch {
        try {
            $webClient = New-Object System.Net.WebClient
            $webClient.DownloadFile($releaseExeUrl, $tempInstaller)
        } catch {
            if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
                try { & curl.exe -sSL $releaseExeUrl -o $tempInstaller } catch {}
            }
        }
    }

    if (Test-Path $tempInstaller) {
        $fileSize = (Get-Item $tempInstaller).Length
        if ($fileSize -gt 500000) {
            Write-Host "[Windows Rice] Pre-compiled rice-installer downloaded successfully ($([math]::Round($fileSize / 1MB, 2)) MB)." -ForegroundColor Green
        } else {
            Remove-Item $tempInstaller -Force -ErrorAction SilentlyContinue
        }
    }
}

# 4. If binary is not available, ensure Scoop & Rust are installed, then compile with Cargo
if (-not (Test-Path $tempInstaller)) {
    Write-Host "[Windows Rice] Preparing build environment (Scoop & Rust)..." -ForegroundColor Cyan

    Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
    $userProfile = $env:USERPROFILE
    $scoopShim = Join-Path $userProfile "scoop\shims"
    $cargoBin = Join-Path $userProfile ".cargo\bin"
    $env:PATH = "$scoopShim;$cargoBin;$env:PATH"

    # Step A: Install Scoop first if missing
    if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
        Write-Host "[Windows Rice] Installing Scoop package manager..." -ForegroundColor Cyan
        try {
            Invoke-Expression (Invoke-RestMethod -Uri https://get.scoop.sh -UseBasicParsing)
        } catch {
            Write-Host "[Windows Rice] Scoop setup notice: $_" -ForegroundColor Yellow
        }
        $env:PATH = "$scoopShim;$cargoBin;$env:PATH"
    }

    # Step B: Install Rust via Scoop
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        if (Get-Command scoop -ErrorAction SilentlyContinue) {
            Write-Host "[Windows Rice] Installing Rust toolchain via Scoop..." -ForegroundColor Cyan
            if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
                & scoop install git 2>$null
            }
            & scoop install rust
            $env:PATH = "$scoopShim;$cargoBin;$env:PATH"
        }

        # Direct rustup fallback if scoop didn't provide cargo
        if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
            Write-Host "[Windows Rice] Bootstrapping Rust compiler via rustup..." -ForegroundColor Cyan
            $rustupInit = Join-Path $env:TEMP "rustup-init.exe"
            try {
                Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit -UseBasicParsing -ErrorAction Stop
                & $rustupInit -y --default-toolchain stable
                $env:PATH = "$cargoBin;$env:PATH"
            } catch {
                Write-Host "[Windows Rice] Rustup download notice: $_" -ForegroundColor Yellow
            }
        }
    }

    # Step C: Build the release binary using Rust
    if (Get-Command cargo -ErrorAction SilentlyContinue -and (Test-Path $manifestPath)) {
        Write-Host "[Windows Rice] Building WINDOW-RICE installer binary (release mode)..." -ForegroundColor Cyan
        Push-Location $installerDir
        try {
            & cargo build --release
        } finally {
            Pop-Location
        }
        if (Test-Path $builtExe) {
            Copy-Item $builtExe $tempInstaller -Force -ErrorAction SilentlyContinue
        }
    }
}

# 5. Launch the binary in the terminal
$binaryToLaunch = if (Test-Path $tempInstaller) { $tempInstaller } elseif (Test-Path $builtExe) { $builtExe } else { $null }

if ($binaryToLaunch) {
    Write-Host "[Windows Rice] Launching WINDOW-RICE Ratatui TUI installer..." -ForegroundColor Green
    & $binaryToLaunch $extraArgs
    exit $LASTEXITCODE
} else {
    Write-Host "[Windows Rice] Error: Failed to acquire or build rice-installer.exe binary." -ForegroundColor Red
    exit 1
}

# SIG # Begin signature block
# MIIGKwYJKoZIhvcNAQcCoIIGHDCCBhgCAQExDzANBglghkgBZQMEAgEFADB5Bgor
# BgEEAYI3AgEEoGswaTA0BgorBgEEAYI3AgEeMCYCAwEAAAQQH8w7YFlLCE63JNLG
# KX7zUQIBAAIBAAIBAAIBAAIBADAxMA0GCWCGSAFlAwQCAQUABCAVboY4EXmhMUt0
# o8eykFqm6QtvCVWrRf3qZLrNYryBVKCCA24wggNqMIICUqADAgECAhARUGl3tg6h
# n0/ys9hv8thvMA0GCSqGSIb3DQEBCwUAME0xHTAbBgNVBAsMFFdpbmRvdy1SaWNl
# IFNlY3VyaXR5MQ8wDQYDVQQKDAZIS0RFVlMxGzAZBgNVBAMMEkhLREVWUyBXaW5k
# b3ctUmljZTAeFw0yNjEwMjUxOTQ0MTBaFw0zMTEwMjUxOTU0MDlaME0xHTAbBgNV
# BAsMFFdpbmRvdy1SaWNlIFNlY3VyaXR5MQ8wDQYDVQQKDAZIS0RFVlMxGzAZBgNV
# BAMMEkhLREVWUyBXaW5kb3ctUmljZTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCC
# AQoCggEBAKpMfjIWf4k5efsZ1Hgm44zSQ774gIOgLUlex8pCUOONd8CDPI3flSTP
# IOGRL1v31kOkWrck0M98/vZNZsg8f7wUt+594e0xm81Hv7nVDLJJo8bVVlfji8HX
# 1LqMyLZejHCG9LZ6OSACOma1+ZonjEGugwfyr5Z+GHzbLZJyjaN5PnmuRQUwAwUc
# spcatF6pD4wRtXv/rRrn5MgH3AsxZ+kP0Lq4zN1ZHwUa6GQiqMCu7tdeISOf7e2V
# 0NsVNbpoO1zyE85DAoVNPP8pzU1zN8iofUJCyEfCB5ReXqi32PaNNUqKKmJR7r4W
# PFrrx3+V8Fq8bv/xDz78e1sV5siFYcECAwEAAaNGMEQwDgYDVR0PAQH/BAQDAgeA
# MBMGA1UdJQQMMAoGCCsGAQUFBwMDMB0GA1UdDgQWBBTaR2HQjO53gOdME6JM5TuL
# v3ZHkTANBgkqhkiG9w0BAQsFAAOCAQEAPDzOoHcNyhqB8amMMvJWT12jrn2JpHSP
# Lwks/m776zAGKjgZGQzUbn+/J85/fJnmlEqqF/xC8SjZq5MRIjsR2ypmajbRvRo2
# 1E+tzu+oOs8C+mMS/7Yz51ooOjtwz2Qyt4cukIggbtbxi2TgcIwbPPXZ9RzD8iVc
# 8zh+m3/3Dy5PUy6IhstK8Y0FbIQzA2bByR/h6smd+3FWf3n5egui9cmc9dWIcegO
# mQNr2RsRzLnbaz7AYGPj90HXOGT3H2TGJ6Vj5lsv75BCawpSLmr5aaiIMKgEeh2D
# 9nJaDSAn7rMmyIRmZVZlXDtIu5UrdPIlhf0AJ6+TMG+J4yGyWzcSJDGCAhMwggIP
# AgEBMGEwTTEdMBsGA1UECwwUV2luZG93LVJpY2UgU2VjdXJpdHkxDzANBgNVBAoM
# BkhLREVWUzEbMBkGA1UEAwwSSEtERVZTIFdpbmRvdy1SaWNlAhARUGl3tg6hn0/y
# s9hv8thvMA0GCWCGSAFlAwQCAQUAoIGEMBgGCisGAQQBgjcCAQwxCjAIoAKAAKEC
# gAAwGQYJKoZIhvcNAQkDMQwGCisGAQQBgjcCAQQwHAYKKwYBBAGCNwIBCzEOMAwG
# CisGAQQBgjcCARUwLwYJKoZIhvcNAQkEMSIEIORDFVbB/TaitcwbgsyuhvumP00O
# d6RTJggQFeBPNfG4MA0GCSqGSIb3DQEBAQUABIIBAGZflQKAJ6o3pbVwgtJI5Kcy
# Klamic1+0QrXkMsVzidKU5ELu3HFcp2Gv1KZD7YFWgYQESkA/q/9DgOzIzaaMprK
# XbkTsFvKIMLh7MyDhAi2W8P32lumjvH2hiVn6UrkMm2MqJyr38ggjIAbyHXpMNhR
# i/z+E+Z1qXIGHtlpchq+lhkW9MXHJuDCbiBmdVG+HfR9fe961nY44NzCA0BfwR9D
# QZKL4+NouBnukUqgdEDMhCokUn5ldhCbPD0gM16XWsdDYYLEBPk9ZV1fbkSO/523
# SABqvhgXd2X0VA3A63SgrY6iuuItZ/HZp7EYoynx53MW+FAW3mRUQRCihKw0TNs=
# SIG # End signature block
