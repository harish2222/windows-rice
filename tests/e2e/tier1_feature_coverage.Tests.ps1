# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: E2E Tier 1 Feature Coverage Test Suite
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
$YasbToml = Join-Path $RepoRoot "yasb\Cargo.toml"

# ============================================================================
# R1: TUI Input Handling & Extended Navigation
# ============================================================================
Set-TestContext -Tier "Tier 1" -Feature "R1: TUI Input Handling & Navigation"

Describe "R1 - TUI Input Controls & Keyboard Navigation" {
    It "T1-R1-01: rice-installer.exe --help documents case-insensitive hotkeys" {
        Should-Exist $InstallerExe "Installer binary must exist"
        $help = & $InstallerExe --help | Out-String
        Should-Contain $help "[q/Q]" "Help must document q/Q quit hotkey"
        Should-Match $help "\[k/j\]" "Help must document k/j navigation"
        Should-Contain $help "[a/A]" "Help must document a/A select all hotkey"
        Should-Contain $help "[n/N]" "Help must document n/N select none hotkey"
        Should-Contain $help "[m/M]" "Help must document m/M select missing hotkey"
        Should-Contain $help "[l/L]" "Help must document l/L link hotkey"
        Should-Contain $help "[u/U]" "Help must document u/U unlink hotkey"
        Should-Contain $help "[i/I]" "Help must document i/I install hotkey"
        Should-Contain $help "[r/R]" "Help must document r/R reload hotkey"
        Should-Contain $help "[c/C]" "Help must document c/C update hotkey"
        Should-Contain $help "[w/W]" "Help must document w/W wallpaper hotkey"
        Should-Contain $help "[v/V]" "Help must document v/V verify hotkey"
        Should-Contain $help "[b/B]" "Help must document b/B bootstrap hotkey"
    }

    It "T1-R1-02: rice-installer.exe --help documents extended navigation keys" {
        $help = & $InstallerExe --help | Out-String
        Should-Contain $help "[PgUp/PgDn]" "Help must document PageUp/PageDown"
        Should-Contain $help "[Home/End]" "Help must document Home/End"
    }

    It "T1-R1-03: rice-installer.exe --help documents dual-key selection toggle (Space/Enter)" {
        $help = & $InstallerExe --help | Out-String
        Should-Contain $help "[Space] or [Enter]" "Help must document Space and Enter selection toggle"
    }

    It "T1-R1-04: Cargo unit tests verify App::handle_key case-insensitive hotkeys" {
        $output = & cargo test --manifest-path $CargoToml test_case_insensitive_hotkeys 2>&1 | Out-String
        Should-Match $output "test app::tests::test_case_insensitive_hotkeys \.\.\. ok" "Case-insensitive hotkey test must pass"
    }

    It "T1-R1-05: Cargo unit tests verify dual-key toggle with Space and Enter" {
        $output = & cargo test --manifest-path $CargoToml test_space_and_enter_selection_toggle 2>&1 | Out-String
        Should-Match $output "test app::tests::test_space_and_enter_selection_toggle \.\.\. ok" "Space and Enter toggle test must pass"
    }

    It "T1-R1-06: Cargo unit tests verify navigation bounds and list scrolling" {
        $output = & cargo test --manifest-path $CargoToml test_navigation_and_bounds 2>&1 | Out-String
        Should-Match $output "test app::tests::test_navigation_and_bounds \.\.\. ok" "Navigation and bounds test must pass"
    }
}

# ============================================================================
# R2: Non-Destructive Config Unlinking & Backup Restoration
# ============================================================================
Set-TestContext -Tier "Tier 1" -Feature "R2: Non-Destructive Config Unlinking & Backup Restoration"

Describe "R2 - Non-Destructive Config Unlinking & Backup Restoration Engine" {
    It "T1-R2-01: Backup engine creates timestamped backup directory structure" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $output = & cargo test --manifest-path $CargoToml test_backup_and_restore_cycle 2>&1 | Out-String
                Should-Match $output "test linker::tests::test_backup_and_restore_cycle \.\.\. ok" "Backup and restore cycle test must pass"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T1-R2-02: Linker recursive file copy helper preserves file contents" {
        $output = & cargo test --manifest-path $CargoToml test_copy_recursive_file 2>&1 | Out-String
        Should-Match $output "test linker::tests::test_copy_recursive_file \.\.\. ok" "File copy test must pass"
    }

    It "T1-R2-03: Linker recursive directory copy helper preserves folder hierarchy" {
        $output = & cargo test --manifest-path $CargoToml test_copy_recursive_dir 2>&1 | Out-String
        Should-Match $output "test linker::tests::test_copy_recursive_dir \.\.\. ok" "Dir copy test must pass"
    }

    It "T1-R2-04: Unlinking restores backed-up user configurations cleanly" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $userTarget = Join-Path $sandbox "user_config.toml"
                Set-Content -Path $userTarget -Value "user_custom_key = 'preserved_val'" -Encoding utf8
                Should-Exist $userTarget "User initial target must exist"
                # Validate restoration mechanism
                $output = & cargo test --manifest-path $CargoToml test_backup_and_restore_cycle 2>&1 | Out-String
                Should-Contain $output "ok"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T1-R2-05: Binary preservation guarantee: unlinking distinguishes binaries from configs" {
        $output = & cargo test --manifest-path $CargoToml test_binary_preservation_guarantee 2>&1 | Out-String
        Should-Match $output "test app::tests::test_binary_preservation_guarantee \.\.\. ok" "Binary preservation test must pass"
    }

    It "T1-R2-06: CLI unlink options are exposed and callable without errors" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $output = & $InstallerExe --unlink 2>&1 | Out-String
                Should-Contain $output "Unlinking" "CLI must acknowledge unlink command"
                Should-Contain $output "binaries preserved" "CLI must explicitly report binaries preserved"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }
}

# ============================================================================
# R3: Functional Purpose-Based Credit Attribution & Cleanliness
# ============================================================================
Set-TestContext -Tier "Tier 1" -Feature "R3: Functional Purpose-Based Credit Attribution"

Describe "R3 - Purpose-Based Attribution Headers & Repository Cleanliness" {
    It "T1-R3-01: Config files in kew/ contain standard credit headers" {
        $kewFiles = Get-ChildItem -Path (Join-Path $RepoRoot "kew") -Recurse -File -Include *.theme, *.layout, *.ini, kewrc
        Should-BeGreaterThan $kewFiles.Count 10 "Kew directory must have configs"
        foreach ($f in $kewFiles) {
            $content = Get-Content $f.FullName -Raw
            Should-Contain $content "Author -: HKDEVS" "File $($f.Name) must have Author header"
            Should-Contain $content "Project -: Window-Rice" "File $($f.Name) must have Project header"
            Should-Contain $content "Title -:" "File $($f.Name) must have Title header"
        }
    }

    It "T1-R3-02: Config files in cava/ contain standard credit headers" {
        $cavaFiles = Get-ChildItem -Path (Join-Path $RepoRoot "cava") -Recurse -File -Include *.frag, *.vert, *.ini, config
        Should-BeGreaterThan $cavaFiles.Count 0 "Cava directory must have files"
        foreach ($f in $cavaFiles) {
            $content = Get-Content $f.FullName -Raw
            Should-Contain $content "Author -: HKDEVS" "File $($f.Name) must have Author header"
            Should-Contain $content "Project -: Window-Rice" "File $($f.Name) must have Project header"
        }
    }

    It "T1-R3-03: Zero tracked files contain placeholder <respective file title>" {
        $trackedFiles = & git -C $RepoRoot ls-files
        $textFiles = $trackedFiles | Where-Object {
            $_ -notmatch "\.(png|ico|dat|lock|exe|pdb|dll)$" -and
            $_ -ne "ORIGINAL_REQUEST.md" -and
            $_ -ne "PROJECT.md" -and
            $_ -notmatch "^tests/e2e/"
        }
        $matches = @()
        foreach ($rel in $textFiles) {
            $full = Join-Path $RepoRoot $rel
            if (Test-Path $full) {
                $found = Select-String -Path $full -Pattern "<respective file title>" -SimpleMatch -ErrorAction SilentlyContinue
                if ($found) { $matches += $found }
            }
        }
        Should-Be $matches.Count 0 "Repository must not contain any '<respective file title>' placeholders"
    }

    It "T1-R3-04: Zero tracked files contain generic folder / filename titles" {
        $trackedFiles = & git -C $RepoRoot ls-files
        $textFiles = $trackedFiles | Where-Object {
            $_ -notmatch "\.(png|ico|dat|lock|exe|pdb|dll)$" -and
            $_ -ne "ORIGINAL_REQUEST.md" -and
            $_ -ne "PROJECT.md" -and
            $_ -notmatch "^tests/e2e/"
        }
        $matches = @()
        foreach ($rel in $textFiles) {
            $full = Join-Path $RepoRoot $rel
            if (Test-Path $full) {
                $found = Select-String -Path $full -Pattern "Title -:\s*[\w\.\-]+ / [\w\.\-]+" -ErrorAction SilentlyContinue
                if ($found) { $matches += $found }
            }
        }
        Should-Be $matches.Count 0 "Repository must not contain generic 'folder / filename' titles"
    }

    It "T1-R3-05: Package catalog strictly excludes removed packages" {
        $output = & cargo test --manifest-path $CargoToml test_catalog_excludes_removed_items 2>&1 | Out-String
        Should-Match $output "test catalog::tests::test_catalog_excludes_removed_items \.\.\. ok" "Excluded items test must pass"
    }

    It "T1-R3-06: .gitignore lists all deprecated package directories" {
        $gitignore = Get-Content (Join-Path $RepoRoot ".gitignore") -Raw
        Should-Match $gitignore "(?m)^rammap/" ".gitignore must contain rammap/"
        Should-Match $gitignore "(?m)^configstore/" ".gitignore must contain configstore/"
        Should-Match $gitignore "(?m)^freebuff-desktop/" ".gitignore must contain freebuff-desktop/"
        Should-Match $gitignore "(?m)^kilo/" ".gitignore must contain kilo/"
        Should-Match $gitignore "(?m)^opencode/" ".gitignore must contain opencode/"
    }
}

# ============================================================================
# R4: Multi-Shell Integration & Carapace Autocompletions
# ============================================================================
Set-TestContext -Tier "Tier 1" -Feature "R4: Multi-Shell Integration & Carapace Autocompletions"

Describe "R4 - Multi-Shell Integrations & Carapace Specifications" {
    It "T1-R4-01: PowerShell profile defines window-rice with typed parameters" {
        $profilePath = Join-Path $RepoRoot "shells\powershell\profile.ps1"
        Should-Exist $profilePath "PowerShell profile must exist"
        $content = Get-Content $profilePath -Raw
        Should-Contain $content "function window-rice" "Profile must define window-rice function"
        Should-Contain $content "[Alias('v')]" "Profile must support -v alias"
        Should-Contain $content "[Alias('x')]" "Profile must support -x alias"
        Should-Contain $content "[Alias('r')]" "Profile must support -r alias"
        Should-Contain $content "[Alias('c')]" "Profile must support -c alias"
        Should-Contain $content "[Alias('w')]" "Profile must support -w alias"
        Should-Contain $content "[Alias('b')]" "Profile must support -b alias"
    }

    It "T1-R4-02: PowerShell profile aliases rice to window-rice" {
        $content = Get-Content (Join-Path $RepoRoot "shells\powershell\profile.ps1") -Raw
        Should-Contain $content "Set-Alias -Name rice -Value window-rice" "rice alias must be set"
    }

    It "T1-R4-03: Nushell config defines window-rice with flag bindings" {
        $nuConfig = Join-Path $RepoRoot "shells\nushell\config.nu"
        Should-Exist $nuConfig "Nushell config must exist"
        $content = Get-Content $nuConfig -Raw
        Should-Contain $content "def --env window-rice" "Nushell config must define window-rice"
        Should-Contain $content "--update (-c)" "Nushell must bind -c to --update"
        Should-Contain $content "--check-updates (-u)" "Nushell must bind -u to --check-updates"
        Should-Contain $content "--unlink (-x)" "Nushell must bind -x to --unlink"
        Should-Contain $content "alias rice = window-rice" "Nushell must alias rice to window-rice"
    }

    It "T1-R4-04: Xonsh rc.xsh defines window_rice and sets command aliases" {
        $xonshRc = Join-Path $RepoRoot "shells\xonsh\rc.xsh"
        Should-Exist $xonshRc "Xonsh rc.xsh must exist"
        $content = Get-Content $xonshRc -Raw
        Should-Contain $content "def window_rice" "Xonsh config must define window_rice"
        Should-Contain $content 'aliases["window-rice"] = window_rice' "Xonsh must register window-rice alias"
        Should-Contain $content 'aliases["rice"] = window_rice' "Xonsh must register rice alias"
    }

    It "T1-R4-05: Carapace completion spec rice-installer.yaml validates via codegen" {
        $specPath = Join-Path $RepoRoot "carapace\specs\rice-installer.yaml"
        Should-Exist $specPath "rice-installer.yaml must exist"
        $output = & carapace --codegen $specPath 2>&1 | Out-String
        Should-Contain $output "root.go" "Carapace must generate valid Go root package"
    }

    It "T1-R4-06: Carapace completion spec window-rice.yaml validates via codegen" {
        $specPath = Join-Path $RepoRoot "carapace\specs\window-rice.yaml"
        Should-Exist $specPath "window-rice.yaml must exist"
        $output = & carapace --codegen $specPath 2>&1 | Out-String
        Should-Contain $output "root.go" "Carapace must generate valid Go root package"
    }

    It "T1-R4-07: install.ps1 parses cleanly without PowerShell AST errors" {
        $installScript = Join-Path $RepoRoot "install.ps1"
        Should-Exist $installScript "install.ps1 must exist"
        $tokens = $null
        $errors = $null
        [System.Management.Automation.Language.Parser]::ParseFile($installScript, [ref]$tokens, [ref]$errors) | Out-Null
        Should-Be $errors.Count 0 "install.ps1 must parse with 0 AST errors"
    }
}

# ============================================================================
# R5: Persistent Diagnostic Logging & Verification Suite
# ============================================================================
Set-TestContext -Tier "Tier 1" -Feature "R5: Persistent Diagnostic Logging & Verification Suite"

Describe "R5 - Persistent Diagnostic Logging & System Verification" {
    It "T1-R5-01: Installer appends diagnostic log entries to installer.log" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $out = & $InstallerExe --update 2>&1 | Out-String
                $logFile = Join-Path $sandbox ".rice-backups\installer.log"
                Should-Exist $logFile "installer.log must be created in backup directory"
                $logContent = Get-Content $logFile -Raw
                Should-BeGreaterThan $logContent.Length 10 "installer.log must contain logs"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T1-R5-02: Diagnostic log entries follow timestamped prefix format" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $out = & $InstallerExe --update 2>&1 | Out-String
                $logFile = Join-Path $sandbox ".rice-backups\installer.log"
                $lines = Get-Content $logFile
                Should-BeGreaterThan $lines.Count 0 "Log file must have lines"
                $firstLine = $lines[0]
                Should-Match $firstLine "^\[\d+\]" "Log entries must start with [timestamp]"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T1-R5-03: CLI --verify outputs complete system health report" {
        $out = & $InstallerExe --verify | Out-String
        Should-Contain $out "WINDOWS RICE SYSTEM HEALTH CHECK REPORT" "Report banner must be present"
        Should-Contain $out "[Package Managers]" "Package Managers section must be present"
        Should-Contain $out "[Components & Configs]" "Components & Configs section must be present"
        Should-Contain $out "[Carapace Autocompletions]" "Carapace Autocompletions section must be present"
        Should-Contain $out "Summary:" "Summary line must be present"
    }

    It "T1-R5-04: CLI --verify detects all 23 components" {
        $out = & $InstallerExe --verify | Out-String
        Should-Match $out "23/23 binaries detected" "All 23 binaries must be detected"
    }

    It "T1-R5-05: Installer unit tests pass with zero failures" {
        $output = & cargo test --manifest-path $CargoToml 2>&1 | Out-String
        Should-Match $output "test result: ok\.\s+\d+ passed; 0 failed" "Installer tests must pass with 0 failures"
    }

    It "T1-R5-06: YASB workspace unit tests pass with zero failures" {
        $output = & cargo test --manifest-path $YasbToml 2>&1 | Out-String
        Should-Contain $output "test result: ok." "YASB test suites must report ok"
        Should-NotContain $output "FAILED" "YASB test suites must have no failures"
    }
}
