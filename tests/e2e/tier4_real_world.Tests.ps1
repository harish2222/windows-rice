# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: E2E Tier 4 Real-World Application Scenarios Test Suite
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

Set-TestContext -Tier "Tier 4" -Feature "Real-World Application Scenarios"

Describe "Tier 4 - Real-World End-to-End User Workflows" {
    It "T4-SCEN-01: Complete Turnkey Rice Bootstrap, Verify, and Teardown Lifecycle" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                # 1. Verification initial check
                $vOut = & $InstallerExe --verify | Out-String
                Should-Contain $vOut "WINDOWS RICE SYSTEM HEALTH CHECK REPORT" "Initial verify must run"

                # 2. Check update status
                $uOut = & $InstallerExe --update 2>&1 | Out-String
                Should-Contain $uOut "Checking for repository updates" "Update check must run"

                # 3. Teardown / Unlink
                $unOut = & $InstallerExe --unlink-all 2>&1 | Out-String
                Should-Contain $unOut "Unlinking all configuration files" "Unlink all must run"
                Should-Contain $unOut "binaries preserved" "Must preserve binaries"

                # 4. Diagnostic log verification
                $logFile = Join-Path $sandbox ".rice-backups\installer.log"
                Should-Exist $logFile "installer.log must record the turnkey workflow"
                $lines = Get-Content $logFile
                Should-BeGreaterThan $lines.Count 1 "Log file must have recorded multiple events"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T4-SCEN-02: End-to-End Non-Destructive Binary Preservation Audit" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                # Mock a user Scoop directory with binaries
                $scoopApps = Join-Path $sandbox "scoop\apps"
                $glazeBinDir = Join-Path $scoopApps "glazewm\current"
                New-Item -ItemType Directory -Path $glazeBinDir -Force | Out-Null
                $mockExe = Join-Path $glazeBinDir "glazewm.exe"
                [System.IO.File]::WriteAllText($mockExe, "BINARY_DATA_GLAZEWM")

                # Mock PATH
                $oldPath = $env:PATH
                $env:PATH = "$glazeBinDir;$oldPath"
                try {
                    # Run full unlink cycle
                    $out = & $InstallerExe --unlink-all 2>&1 | Out-String
                    Should-Contain $out "binaries preserved"
                    Should-Exist $mockExe "Installed binary in Scoop directory must never be deleted"
                    $contents = (Get-Content $mockExe -Raw).Trim()
                    Should-Be $contents "BINARY_DATA_GLAZEWM" "Binary payload must be completely unmodified"
                } finally {
                    $env:PATH = $oldPath
                }
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T4-SCEN-03: Repository-Wide Purpose-Based Attribution & Cleanliness Audit" {
        $trackedFiles = & git -C $RepoRoot ls-files
        $configDirs = @("cava", "cliamp", "kew", "otty", "fastfetch", "glazewm", "komorebi", "yasb", "carapace", "shells")
        
        $filesToAudit = $trackedFiles | Where-Object {
            $rel = $_
            $inTargetDir = $false
            foreach ($dir in $configDirs) {
                if ($rel -like "$dir/*") { $inTargetDir = $true; break }
            }
            $inTargetDir -and
            ($rel -notmatch "Modules/") -and
            ($rel -notmatch "fastfetch/logos/") -and
            ($rel -ne "fastfetch/sharingan.txt") -and
            ($rel -notmatch "yasb/assets/motifs/") -and
            ($rel -ne "yasb/yasb_colors.css") -and
            ($rel -notmatch "\.(png|ico|dat|lock|exe|pdb|dll|sock|db|svg|xml|winmd|psd1|ps1xml|cat|cose|p7s|sha256)$")
        }

        Should-BeGreaterThan $filesToAudit.Count 50 "Should find more than 50 config files to audit"

        $missingHeaders = @()
        $genericTitles = @()
        foreach ($rel in $filesToAudit) {
            $full = Join-Path $RepoRoot $rel
            if (Test-Path $full) {
                $content = Get-Content $full -Raw -ErrorAction SilentlyContinue
                if ($content) {
                    if ($content -notmatch "Author -:\s*HKDEVS") {
                        $missingHeaders += "$rel (missing Author)"
                    }
                    if ($content -match "<respective file title>") {
                        $genericTitles += "$rel (contains placeholder)"
                    }
                }
            }
        }

        Should-Be $missingHeaders.Count 0 "All config files must have Author header"
        Should-Be $genericTitles.Count 0 "No config files can contain placeholder titles"
    }

    It "T4-SCEN-04: Multi-Shell Integration Parity (PowerShell, Nushell, Xonsh)" {
        # PowerShell
        $psContent = Get-Content (Join-Path $RepoRoot "shells\powershell\profile.ps1") -Raw
        Should-Contain $psContent "function window-rice" "PowerShell must implement window-rice"
        Should-Contain $psContent "Set-Alias -Name rice -Value window-rice" "PowerShell must alias rice"

        # Nushell
        $nuContent = Get-Content (Join-Path $RepoRoot "shells\nushell\config.nu") -Raw
        Should-Contain $nuContent "def --env window-rice" "Nushell must implement window-rice"
        Should-Contain $nuContent "alias rice = window-rice" "Nushell must alias rice"

        # Xonsh
        $xContent = Get-Content (Join-Path $RepoRoot "shells\xonsh\rc.xsh") -Raw
        Should-Contain $xContent "def window_rice" "Xonsh must implement window_rice"
        Should-Contain $xContent 'aliases["window-rice"] = window_rice' "Xonsh must alias window-rice"
        Should-Contain $xContent 'aliases["rice"] = window_rice' "Xonsh must alias rice"
    }

    It "T4-SCEN-05: Carapace Specification Codegen & Completion Schema Validation" {
        $specs = @(
            (Join-Path $RepoRoot "carapace\specs\rice-installer.yaml"),
            (Join-Path $RepoRoot "carapace\specs\window-rice.yaml")
        )
        foreach ($spec in $specs) {
            Should-Exist $spec "Carapace spec must exist: $spec"
            $cg = & carapace --codegen $spec 2>&1 | Out-String
            Should-Contain $cg "root.go" "Carapace must compile valid Go specification"
        }
    }

    It "T4-SCEN-06: Comprehensive System Health Check Audit Integrity" {
        $report = & $InstallerExe --verify | Out-String
        Should-Contain $report "WINDOWS RICE SYSTEM HEALTH CHECK REPORT" "Must contain banner"
        
        # 8 Package managers
        $pms = @("Scoop", "Winget", "Chocolatey", "Cargo", "UV (Python)", "Vcpkg (C++)", "Python", "NPM")
        foreach ($pm in $pms) {
            Should-Contain $report "${pm}:" "Verifier must check package manager $pm"
        }

        # 23 Components
        $comps = @("GlazeWM", "Komorebi", "YASB Status Bar", "Windows Terminal", "Otty Terminal", 
                   "WezTerm", "Alacritty", "PowerShell 7", "Nushell", "Xonsh", "Oh My Posh", 
                   "Carapace Completer", "Zellij Multiplexer", "Kew Music Player", "Fastfetch")
        foreach ($c in $comps) {
            Should-Contain $report $c "Verifier must report component $c"
        }

        # Carapace count
        Should-Contain $report "63 custom YAML specs loaded" "Must report 63 carapace specs"
        Should-Contain $report "23/23 binaries detected" "Must report 23/23 binaries detected"
    }
}
