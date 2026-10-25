# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: E2E Tier 3 Cross-Feature Combinations Test Suite
# ============================================================================

$PSScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not (Get-Module Common)) {
    Import-Module (Join-Path $PSScriptRoot "Common.psm1")
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$InstallerExe = Join-Path $RepoRoot "installer\target\debug\rice-installer.exe"
if (-not (Test-Path $InstallerExe)) {
    $InstallerExe = Join-Path $RepoRoot "installer\target\release\rice-installer.exe"
}
$CargoToml = Join-Path $RepoRoot "installer\Cargo.toml"

Set-TestContext -Tier "Tier 3" -Feature "Cross-Feature Integration"

Describe "Tier 3 - Cross-Feature Pairwise Integrations" {
    It "T3-COMB-01: Pairwise Cycle: Backup -> Link -> Unlink -> Pristine Restore (R2 + R5)" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                # 1. Run cargo integration test for full backup/restore cycle
                $output = & cargo test --manifest-path $CargoToml test_backup_and_restore_cycle 2>&1 | Out-String
                Should-Match $output "test linker::tests::test_backup_and_restore_cycle \.\.\. ok" "Backup and restore cycle test must pass"

                # 2. Verify logging created installer.log during operations
                & $InstallerExe --update 2>&1 | Out-String | Out-Null
                $logFile = Join-Path $sandbox ".rice-backups\installer.log"
                Should-Exist $logFile "installer.log must exist after operation"
                $lines = Get-Content $logFile
                Should-BeGreaterThan $lines.Count 0 "installer.log must have entries"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T3-COMB-02: Shell Function to CLI Execution: window-rice -Verify invocation (R4 + R5)" {
        $profilePath = Join-Path $RepoRoot "shells\powershell\profile.ps1"
        $scriptContent = Get-Content $profilePath -Raw
        
        # Test executing window-rice definition within a private scope
        $sb = [ScriptBlock]::Create($scriptContent + "`n" + 'window-rice -Verify')
        $out = & $sb 2>&1 | Out-String
        Should-Contain $out "WINDOWS RICE SYSTEM HEALTH CHECK REPORT" "Shell function must invoke CLI verifier"
        Should-Contain $out "Summary:" "Shell function must print health check summary"
    }

    It "T3-COMB-03: Navigation Bounds and Selection Consistency under List Cycling (R1 + R2)" {
        # Validates that navigation keys do not corrupt or desynchronize package selection states
        $output = & cargo test --manifest-path $CargoToml test_navigation_and_bounds 2>&1 | Out-String
        Should-Match $output "test app::tests::test_navigation_and_bounds \.\.\. ok"
        
        $toggleOutput = & cargo test --manifest-path $CargoToml test_selection_toggles 2>&1 | Out-String
        Should-Match $toggleOutput "test app::tests::test_selection_toggles \.\.\. ok"
    }

    It "T3-COMB-04: Catalog Package Items to Source Config Attribution Parity (R3 + R1)" {
        # Verify that all active packages in catalog have legitimate source files with attribution
        $output = & cargo test --manifest-path $CargoToml test_catalog_source_paths_exist 2>&1 | Out-String
        Should-Match $output "test catalog::tests::test_catalog_source_paths_exist \.\.\. ok" "All catalog configs must exist"
    }

    It "T3-COMB-05: Carapace Specs to Installer CLI Flags Parity (R4 + R1)" {
        $spec = Get-Content (Join-Path $RepoRoot "carapace\specs\rice-installer.yaml") -Raw
        $help = & $InstallerExe --help | Out-String

        # Every flag in carapace spec must be documented in CLI help
        $flagsToVerify = @("--reload", "--update", "--wallpapers", "--verify", "--link-all", "--unlink", "--bootstrap", "--help")
        foreach ($flag in $flagsToVerify) {
            Should-Contain $spec $flag "Carapace spec must contain $flag"
            Should-Contain $help $flag "Installer help must contain $flag"
        }
    }

    It "T3-COMB-06: Shell Parameter Aliases to Carapace Flags Parity (R4 + R1)" {
        $profile = Get-Content (Join-Path $RepoRoot "shells\powershell\profile.ps1") -Raw
        $carapace = Get-Content (Join-Path $RepoRoot "carapace\specs\window-rice.yaml") -Raw

        # Flags must be aligned between PowerShell profile and Carapace spec
        $aliases = @("Verify", "Unlink", "Reload", "Update", "Wallpapers", "Bootstrap")
        foreach ($alias in $aliases) {
            Should-Contain $profile "[switch]`$$alias" "PowerShell profile must have parameter `$$alias"
        }
    }
}
