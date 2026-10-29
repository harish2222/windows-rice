# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: E2E Test Suite Core Harness and Assertion Framework
# ============================================================================

$script:AllTestResults = [System.Collections.Generic.List[PSCustomObject]]::new()
$script:CurrentTier = "General"
$script:CurrentFeature = "General"
$script:CurrentDescribe = ""
$script:CurrentContext = ""

# Non-fatal skip marker: a test can declare itself ineligible for the current
# host (for example CI runners without the full rice installed) without
# poisoning the suite exit code.
class RiceSkipException : System.Exception {
    RiceSkipException([string]$message) : base($message) {}
}

function Skip-Test {
    param([string]$Message)
    throw [RiceSkipException]::new($Message)
}

function Set-TestContext {
    param(
        [string]$Tier = "General",
        [string]$Feature = "General"
    )
    $script:CurrentTier = $Tier
    $script:CurrentFeature = $Feature
}

function Reset-TestResults {
    $script:AllTestResults.Clear()
}

function Describe {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Body
    )
    $script:CurrentDescribe = $Name
    Write-Host "`n=== [DESCRIBE] $Name ===" -ForegroundColor Cyan
    & $Body
}

function Context {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Body
    )
    $script:CurrentContext = $Name
    Write-Host "  --- [CONTEXT] $Name ---" -ForegroundColor DarkCyan
    & $Body
}

function It {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Test
    )

    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $passed = $false
    $skipped = $false
    $err = $null
    $oldEap = $ErrorActionPreference

    try {
        $ErrorActionPreference = "Continue"
        & $Test
        $passed = $true
    } catch [RiceSkipException] {
        $passed = $true
        $skipped = $true
        $err = $_
    } catch {
        $passed = $false
        $err = $_
    } finally {
        $ErrorActionPreference = $oldEap
        $stopwatch.Stop()
    }

    $result = [PSCustomObject]@{
        Tier        = $script:CurrentTier
        Feature     = $script:CurrentFeature
        Describe    = $script:CurrentDescribe
        Context     = $script:CurrentContext
        Name        = $Name
        Passed      = $passed
        Skipped     = $skipped
        Error       = if ($err) { $err.ToString() } else { $null }
        DurationMs  = $stopwatch.ElapsedMilliseconds
    }

    $script:AllTestResults.Add($result)

    if ($skipped) {
        Write-Host "    [SKIP] $Name ($($result.DurationMs)ms) - $($err.Exception.Message)" -ForegroundColor DarkYellow
    } elseif ($passed) {
        Write-Host "    [PASS] $Name ($($result.DurationMs)ms)" -ForegroundColor Green
    } else {
        Write-Host "    [FAIL] $Name ($($result.DurationMs)ms)" -ForegroundColor Red
        if ($err) {
            Write-Host "           Error: $($err.ToString())" -ForegroundColor DarkRed
        }
    }
}

# --- Assertion Helpers ---

function Should-Be {
    param(
        [Parameter(Mandatory = $false)]$Actual,
        [Parameter(Mandatory = $false)]$Expected,
        [string]$Message = ""
    )
    if ($Actual -ne $Expected) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected: '$Expected', but got: '$Actual'"
    }
}

function Should-NotBe {
    param(
        [Parameter(Mandatory = $false)]$Actual,
        [Parameter(Mandatory = $false)]$Expected,
        [string]$Message = ""
    )
    if ($Actual -eq $Expected) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected value NOT to be: '$Expected', but it was"
    }
}

function Should-Exist {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string]$Message = ""
    )
    if (-not (Test-Path $Path)) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected path to exist, but not found: '$Path'"
    }
}

function Should-NotExist {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string]$Message = ""
    )
    if (Test-Path $Path) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected path NOT to exist, but found: '$Path'"
    }
}

function Should-Match {
    param(
        [Parameter(Mandatory = $false)][string]$Actual,
        [Parameter(Mandatory = $true)][string]$Pattern,
        [string]$Message = ""
    )
    if ($Actual -notmatch $Pattern) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected string to match pattern '$Pattern', but got: '$Actual'"
    }
}

function Should-Contain {
    param(
        [Parameter(Mandatory = $false)]$Collection,
        [Parameter(Mandatory = $true)]$Item,
        [string]$Message = ""
    )
    $contains = $false
    if ($Collection -is [string]) {
        $contains = $Collection.Contains([string]$Item)
    } elseif ($Collection -is [System.Collections.IEnumerable]) {
        foreach ($el in $Collection) {
            if ($el -eq $Item) { $contains = $true; break }
        }
    }
    if (-not $contains) {
        $msg = if ($Message) { "$Message - " } else { "" }
        if ($Collection -is [string]) {
            throw "${msg}Expected string to contain '$Item', but got: '$Collection'"
        }
        throw "${msg}Expected collection to contain '$Item'"
    }
}

function Should-NotContain {
    param(
        [Parameter(Mandatory = $false)]$Collection,
        [Parameter(Mandatory = $true)]$Item,
        [string]$Message = ""
    )
    $contains = $false
    if ($Collection -is [string]) {
        $contains = $Collection.Contains([string]$Item)
    } elseif ($Collection -is [System.Collections.IEnumerable]) {
        foreach ($el in $Collection) {
            if ($el -eq $Item) { $contains = $true; break }
        }
    }
    if ($contains) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected collection/string NOT to contain '$Item', but it did"
    }
}

function Should-BeGreaterThan {
    param(
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)]$Threshold,
        [string]$Message = ""
    )
    if ($Actual -le $Threshold) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected $Actual to be greater than $Threshold"
    }
}

function Should-BeLessThan {
    param(
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)]$Threshold,
        [string]$Message = ""
    )
    if ($Actual -ge $Threshold) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected $Actual to be less than $Threshold"
    }
}

function Should-Throw {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$Action,
        [string]$Message = ""
    )
    $threw = $false
    try {
        & $Action
    } catch {
        $threw = $true
    }
    if (-not $threw) {
        $msg = if ($Message) { "$Message - " } else { "" }
        throw "${msg}Expected action to throw an exception, but it succeeded"
    }
}

# --- Sandboxing & Isolation ---

function New-TestSandbox {
    $baseTemp = [System.IO.Path]::GetTempPath()
    $sandboxName = "rice_sandbox_" + [Guid]::NewGuid().ToString("N")
    $sandboxPath = Join-Path $baseTemp $sandboxName
    New-Item -ItemType Directory -Path $sandboxPath -Force | Out-Null
    return $sandboxPath
}

function Remove-TestSandbox {
    param([string]$Path)
    if ($Path -and (Test-Path $Path)) {
        try {
            Remove-Item -Path $Path -Recurse -Force -ErrorAction SilentlyContinue
        } catch {}
    }
}

function Invoke-WithIsolatedProfile {
    param(
        [Parameter(Mandatory = $true)][string]$SandboxDir,
        [Parameter(Mandatory = $true)][scriptblock]$Script
    )
    $oldProfile = [Environment]::GetEnvironmentVariable('USERPROFILE', 'Process')
    $oldBackupDir = [Environment]::GetEnvironmentVariable('RICE_BACKUP_DIR', 'Process')
    $oldHome = [Environment]::GetEnvironmentVariable('HOME', 'Process')
    $oldAppData = [Environment]::GetEnvironmentVariable('APPDATA', 'Process')
    $oldLocalAppData = [Environment]::GetEnvironmentVariable('LOCALAPPDATA', 'Process')
    $oldRustupHome = [Environment]::GetEnvironmentVariable('RUSTUP_HOME', 'Process')
    $oldCargoHome = [Environment]::GetEnvironmentVariable('CARGO_HOME', 'Process')

    # Pin the Rust toolchain homes BEFORE the process env is redirected. cargo
    # (run by R2 cargo-cycle tests) resolves its toolchains through rustup, which
    # defaults to %USERPROFILE%\.rustup - once USERPROFILE/HOME point into the
    # sandbox that default goes missing and rustup reports "no default is
    # configured" (CI hit exactly this on the windows runner). Pin the real
    # homes explicitly so the sandbox redirect never degrades them.
    $rustupHome = if ($oldRustupHome) { $oldRustupHome } else { Join-Path $oldProfile ".rustup" }
    $cargoHome = if ($oldCargoHome) { $oldCargoHome } else { Join-Path $oldProfile ".cargo" }

    $backupRoot = Join-Path $SandboxDir ".rice-backups"
    # Virtualize the whole roamed profile surface. APPDATA/LOCALAPPDATA MUST be
    # redirected too: catalog mappings like carapace specs (an AppData junction
    # into the repo) or Windows Terminal settings live there, and failing to
    # redirect them lets a misplaced unlink mutate the LIVE host profile.
    New-Item -ItemType Directory -Path (Join-Path $SandboxDir "AppData\Roaming") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $SandboxDir "AppData\Local") -Force | Out-Null
    $appDataDir = Join-Path $SandboxDir "AppData\Roaming"
    $localAppDataDir = Join-Path $SandboxDir "AppData\Local"

    [Environment]::SetEnvironmentVariable('USERPROFILE', $SandboxDir, 'Process')
    [Environment]::SetEnvironmentVariable('HOME', $SandboxDir, 'Process')
    [Environment]::SetEnvironmentVariable('RICE_BACKUP_DIR', $backupRoot, 'Process')
    [Environment]::SetEnvironmentVariable('APPDATA', $appDataDir, 'Process')
    [Environment]::SetEnvironmentVariable('LOCALAPPDATA', $localAppDataDir, 'Process')
    [Environment]::SetEnvironmentVariable('RUSTUP_HOME', $rustupHome, 'Process')
    [Environment]::SetEnvironmentVariable('CARGO_HOME', $cargoHome, 'Process')
    $env:USERPROFILE = $SandboxDir
    $env:HOME = $SandboxDir
    $env:RICE_BACKUP_DIR = $backupRoot
    $env:APPDATA = $appDataDir
    $env:LOCALAPPDATA = $localAppDataDir
    $env:RUSTUP_HOME = $rustupHome
    $env:CARGO_HOME = $cargoHome

    try {
        & $Script
    } finally {
        [Environment]::SetEnvironmentVariable('USERPROFILE', $oldProfile, 'Process')
        [Environment]::SetEnvironmentVariable('HOME', $oldHome, 'Process')
        [Environment]::SetEnvironmentVariable('RICE_BACKUP_DIR', $oldBackupDir, 'Process')
        [Environment]::SetEnvironmentVariable('APPDATA', $oldAppData, 'Process')
        [Environment]::SetEnvironmentVariable('LOCALAPPDATA', $oldLocalAppData, 'Process')
        [Environment]::SetEnvironmentVariable('RUSTUP_HOME', $oldRustupHome, 'Process')
        [Environment]::SetEnvironmentVariable('CARGO_HOME', $oldCargoHome, 'Process')
        $env:USERPROFILE = $oldProfile
        $env:HOME = $oldHome
        $env:RICE_BACKUP_DIR = $oldBackupDir
        $env:APPDATA = $oldAppData
        $env:LOCALAPPDATA = $oldLocalAppData
        $env:RUSTUP_HOME = $oldRustupHome
        $env:CARGO_HOME = $oldCargoHome
    }
}

# --- Installer Binary Selection ---

function Select-InstallerExe {
    # Picks rice-installer.exe by FRESHEST build time, never blindly debug-first:
    # a stale debug binary silently running old engine code deleted the live
    # carapace specs once during a tier4 run (this exact hazard). Fails loudly
    # when no binary exists so CI/local runs rebuild instead of guessing.
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot
    )
    $debugExe = Join-Path $RepoRoot "installer\target\debug\rice-installer.exe"
    $releaseExe = Join-Path $RepoRoot "installer\target\release\rice-installer.exe"

    $debugOk = Test-Path $debugExe
    $releaseOk = Test-Path $releaseExe
    if (-not $debugOk -and -not $releaseOk) {
        throw "rice-installer.exe not found. Build it first: cargo build --manifest-path installer/Cargo.toml"
    }
    if ($debugOk -and $releaseOk) {
        $debugTime = (Get-Item $debugExe).LastWriteTimeUtc
        $releaseTime = (Get-Item $releaseExe).LastWriteTimeUtc
        if ($debugTime -ge $releaseTime) { return $debugExe } else { return $releaseExe }
    }
    if ($debugOk) { return $debugExe }
    return $releaseExe
}

# --- Reporting ---

function Get-TestResults {
    return $script:AllTestResults
}

function Get-TestSummary {
    $total = $script:AllTestResults.Count
    $passed = ($script:AllTestResults | Where-Object { $_.Passed -and -not $_.Skipped }).Count
    $skipped = ($script:AllTestResults | Where-Object { $_.Skipped }).Count
    $failed = ($script:AllTestResults | Where-Object { -not $_.Passed -and -not $_.Skipped }).Count
    $duration = ($script:AllTestResults | Measure-Object -Property DurationMs -Sum).Sum

    $byTier = $script:AllTestResults | Group-Object -Property Tier | ForEach-Object {
        [PSCustomObject]@{
            Tier    = $_.Name
            Total   = $_.Count
            Passed  = ($_.Group | Where-Object { $_.Passed -and -not $_.Skipped }).Count
            Skipped = ($_.Group | Where-Object { $_.Skipped }).Count
            Failed  = ($_.Group | Where-Object { -not $_.Passed -and -not $_.Skipped }).Count
        }
    }

    $byFeature = $script:AllTestResults | Group-Object -Property Feature | ForEach-Object {
        [PSCustomObject]@{
            Feature = $_.Name
            Total   = $_.Count
            Passed  = ($_.Group | Where-Object { $_.Passed -and -not $_.Skipped }).Count
            Skipped = ($_.Group | Where-Object { $_.Skipped }).Count
            Failed  = ($_.Group | Where-Object { -not $_.Passed -and -not $_.Skipped }).Count
        }
    }

    return [PSCustomObject]@{
        Total      = $total
        Passed     = $passed
        Skipped    = $skipped
        Failed     = $failed
        DurationMs = $duration
        ByTier     = $byTier
        ByFeature  = $byFeature
    }
}

Export-ModuleMember -Function Set-TestContext, Reset-TestResults, Describe, Context, It, Skip-Test, `
    Should-Be, Should-NotBe, Should-Exist, Should-NotExist, Should-Match, Should-Contain, Should-NotContain, `
    Should-BeGreaterThan, Should-BeLessThan, Should-Throw, `
    New-TestSandbox, Remove-TestSandbox, Invoke-WithIsolatedProfile, Select-InstallerExe, `
    Get-TestResults, Get-TestSummary
