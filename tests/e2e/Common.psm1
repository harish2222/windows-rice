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
    $err = $null
    $oldEap = $ErrorActionPreference

    try {
        $ErrorActionPreference = "Continue"
        & $Test
        $passed = $true
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
        Error       = if ($err) { $err.ToString() } else { $null }
        DurationMs  = $stopwatch.ElapsedMilliseconds
    }

    $script:AllTestResults.Add($result)

    if ($passed) {
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
        throw "${msg}Expected collection/string to contain '$Item'"
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

    $backupRoot = Join-Path $SandboxDir ".rice-backups"
    [Environment]::SetEnvironmentVariable('USERPROFILE', $SandboxDir, 'Process')
    [Environment]::SetEnvironmentVariable('HOME', $SandboxDir, 'Process')
    [Environment]::SetEnvironmentVariable('RICE_BACKUP_DIR', $backupRoot, 'Process')
    $env:USERPROFILE = $SandboxDir
    $env:HOME = $SandboxDir
    $env:RICE_BACKUP_DIR = $backupRoot

    try {
        & $Script
    } finally {
        [Environment]::SetEnvironmentVariable('USERPROFILE', $oldProfile, 'Process')
        [Environment]::SetEnvironmentVariable('HOME', $oldHome, 'Process')
        [Environment]::SetEnvironmentVariable('RICE_BACKUP_DIR', $oldBackupDir, 'Process')
        $env:USERPROFILE = $oldProfile
        $env:HOME = $oldHome
        $env:RICE_BACKUP_DIR = $oldBackupDir
    }
}

# --- Reporting ---

function Get-TestResults {
    return $script:AllTestResults
}

function Get-TestSummary {
    $total = $script:AllTestResults.Count
    $passed = ($script:AllTestResults | Where-Object { $_.Passed }).Count
    $failed = $total - $passed
    $duration = ($script:AllTestResults | Measure-Object -Property DurationMs -Sum).Sum

    $byTier = $script:AllTestResults | Group-Object -Property Tier | ForEach-Object {
        [PSCustomObject]@{
            Tier    = $_.Name
            Total   = $_.Count
            Passed  = ($_.Group | Where-Object { $_.Passed }).Count
            Failed  = ($_.Group | Where-Object { -not $_.Passed }).Count
        }
    }

    $byFeature = $script:AllTestResults | Group-Object -Property Feature | ForEach-Object {
        [PSCustomObject]@{
            Feature = $_.Name
            Total   = $_.Count
            Passed  = ($_.Group | Where-Object { $_.Passed }).Count
            Failed  = ($_.Group | Where-Object { -not $_.Passed }).Count
        }
    }

    return [PSCustomObject]@{
        Total      = $total
        Passed     = $passed
        Failed     = $failed
        DurationMs = $duration
        ByTier     = $byTier
        ByFeature  = $byFeature
    }
}

Export-ModuleMember -Function Set-TestContext, Reset-TestResults, Describe, Context, It, `
    Should-Be, Should-NotBe, Should-Exist, Should-NotExist, Should-Match, Should-Contain, Should-NotContain, `
    Should-BeGreaterThan, Should-BeLessThan, Should-Throw, `
    New-TestSandbox, Remove-TestSandbox, Invoke-WithIsolatedProfile, `
    Get-TestResults, Get-TestSummary
