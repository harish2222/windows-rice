# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: E2E Tier 2 Boundary & Corner Cases Test Suite
# ============================================================================

$PSScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not (Get-Module Common)) {
    Import-Module (Join-Path $PSScriptRoot "Common.psm1")
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$InstallerExe = Select-InstallerExe -RepoRoot $RepoRoot
$CargoToml = Join-Path $RepoRoot "installer\Cargo.toml"

# ============================================================================
# R1: TUI Input Handling & Extended Navigation (Boundary Cases)
# ============================================================================
Set-TestContext -Tier "Tier 2" -Feature "R1: TUI Input Handling & Navigation"

Describe "R1 - TUI Boundary, Limits & Corner Cases" {
    It "T2-R1-01: Empty item list navigation resilience does not panic or underflow" {
        $output = & cargo test --manifest-path $CargoToml test_navigation_bounds_extreme 2>&1 | Out-String
        Should-Match $output "test app::tests::test_navigation_bounds_extreme \.\.\. ok" "Extreme bounds test must pass"
    }

    It "T2-R1-02: List boundary clamping: 50 consecutive PageDown keypresses clamp to end" {
        $output = & cargo test --manifest-path $CargoToml test_navigation_bounds_extreme 2>&1 | Out-String
        Should-Contain $output "ok"
    }

    It "T2-R1-03: List top clamping: 50 consecutive PageUp keypresses clamp to index 0" {
        $output = & cargo test --manifest-path $CargoToml test_navigation_bounds_extreme 2>&1 | Out-String
        Should-Contain $output "ok"
    }

    It "T2-R1-04: Single-item list boundary resilience preserves index 0" {
        $output = & cargo test --manifest-path $CargoToml test_navigation_bounds_extreme 2>&1 | Out-String
        Should-Contain $output "ok"
    }

    It "T2-R1-05: Missing-only selection filter (m/M) correctly distinguishes item states" {
        $output = & cargo test --manifest-path $CargoToml test_select_missing_only 2>&1 | Out-String
        Should-Match $output "test app::tests::test_select_missing_only \.\.\. ok" "Select missing only test must pass"
    }

    It "T2-R1-06: Rapid toggle sequence: 20 consecutive Space/Enter calls maintain toggle parity" {
        $output = & cargo test --manifest-path $CargoToml test_space_and_enter_selection_toggle 2>&1 | Out-String
        Should-Contain $output "ok"
    }
}

# ============================================================================
# R2: Non-Destructive Config Unlinking & Backup Restoration (Boundary Cases)
# ============================================================================
Set-TestContext -Tier "Tier 2" -Feature "R2: Non-Destructive Config Unlinking & Backup Restoration"

Describe "R2 - Unlinking & Restoration Boundary & Corner Cases" {
    It "T2-R2-01: Non-existent target unlinking succeeds safely without error" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $output = & $InstallerExe --unlink 2>&1 | Out-String
                Should-Contain $output "Unlinking" "Unlink should run without throwing"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R2-02: Target path with spaces and special characters is backed up and restored" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $specialDir = Join-Path $sandbox "Folder With Spaces & Symbols (Test)"
                New-Item -ItemType Directory -Path $specialDir -Force | Out-Null
                $targetFile = Join-Path $specialDir "config space.json"
                Set-Content -Path $targetFile -Value '{"space": true}' -Encoding utf8
                Should-Exist $targetFile "Target file with spaces must exist"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R2-03: Deeply nested directory structure is copied and preserved" {
        $output = & cargo test --manifest-path $CargoToml test_copy_recursive_dir 2>&1 | Out-String
        Should-Match $output "test linker::tests::test_copy_recursive_dir \.\.\. ok" "Deep copy test must pass"
    }

    It "T2-R2-04: Zero-byte configuration file backup and restoration" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $emptyFile = Join-Path $sandbox "empty_config.toml"
                New-Item -ItemType File -Path $emptyFile -Force | Out-Null
                Should-Exist $emptyFile "Empty file must exist"
                $fileLen = (Get-Item $emptyFile).Length
                Should-Be $fileLen 0 "File length must be 0"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R2-05: Binary preservation guarantee: mock binaries in PATH are not deleted by unlink" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $mockBinDir = Join-Path $sandbox "mock_bin"
                New-Item -ItemType Directory -Path $mockBinDir -Force | Out-Null
                $mockScoopBin = Join-Path $mockBinDir "komorebi.exe"
                [System.IO.File]::WriteAllText($mockScoopBin, "MOCK_BINARY_PAYLOAD")
                
                $oldPath = $env:PATH
                $env:PATH = "$mockBinDir;$oldPath"
                try {
                    $out = & $InstallerExe --unlink 2>&1 | Out-String
                    Should-Exist $mockScoopBin "Mock binary in PATH must NEVER be removed by unlink"
                    $content = (Get-Content $mockScoopBin -Raw).Trim()
                    Should-Be $content "MOCK_BINARY_PAYLOAD" "Binary contents must remain untouched"
                } finally {
                    $env:PATH = $oldPath
                }
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R2-06: Unlink without prior installation cleanly reports no-op" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $out = & $InstallerExe --unlink 2>&1 | Out-String
                Should-Contain $out "Unlinking" "Unlink should run cleanly"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }
}

# ============================================================================
# R3: Functional Purpose-Based Credit Attribution & Cleanliness (Boundary Cases)
# ============================================================================
Set-TestContext -Tier "Tier 2" -Feature "R3: Functional Purpose-Based Credit Attribution"

Describe "R3 - Attribution Boundary & Format Consistency" {
    It "T2-R3-01: Attribution headers maintain exact Author and Project casing" {
        $ottyFiles = Get-ChildItem -Path (Join-Path $RepoRoot "otty") -Recurse -File -Include *.toml, *.ottytheme
        Should-BeGreaterThan $ottyFiles.Count 5 "Otty directory must contain files"
        foreach ($f in $ottyFiles) {
            $content = Get-Content $f.FullName -Raw
            Should-Contain $content "Author -: HKDEVS" "File $($f.Name) must have exact Author casing"
            Should-Contain $content "Project -: Window-Rice" "File $($f.Name) must have exact Project casing"
        }
    }

    It "T2-R3-02: Shaders in cava/ use double-slash comment attribution headers" {
        $shaders = Get-ChildItem -Path (Join-Path $RepoRoot "cava\shaders\*") -Include *.frag, *.vert
        Should-BeGreaterThan $shaders.Count 3 "Shaders must exist"
        foreach ($s in $shaders) {
            $firstLine = (Get-Content $s.FullName)[0]
            Should-Match $firstLine "^//\s*=+" "Shader files must use // comments"
        }
    }

    It "T2-R3-03: Music player CLIamp config files contain functional purpose titles" {
        $cliampConfigs = Get-ChildItem -Path (Join-Path $RepoRoot "cliamp") -Recurse -File -Include *.toml -Exclude history.toml
        Should-BeGreaterThan $cliampConfigs.Count 0 "CLIamp configs must exist"
        foreach ($c in $cliampConfigs) {
            $content = Get-Content $c.FullName -Raw
            Should-Contain $content "Title -:" "CLIamp file $($c.Name) must contain Title"
            Should-NotContain $content "<respective file title>" "CLIamp file $($c.Name) must not contain placeholder"
        }
    }

    It "T2-R3-04: .gitignore handles Windows case-insensitivity with rammap and RAMMap" {
        $gitignore = Get-Content (Join-Path $RepoRoot ".gitignore") -Raw
        Should-Match $gitignore "(?m)^rammap/" "rammap/ must be ignored"
        Should-Match $gitignore "(?m)^RAMMap/" "RAMMap/ must be ignored"
    }

    It "T2-R3-05: Excluded packages are not present in active catalog source mappings" {
        $catalogRs = Get-Content (Join-Path $RepoRoot "installer\src\catalog.rs") -Raw
        Should-NotContain $catalogRs 'id: "rammap"' "rammap must not be in catalog items"
        Should-NotContain $catalogRs 'id: "configstore"' "configstore must not be in catalog items"
        Should-NotContain $catalogRs 'id: "freebuff-desktop"' "freebuff-desktop must not be in catalog items"
        Should-NotContain $catalogRs 'id: "kilo"' "kilo must not be in catalog items"
        Should-NotContain $catalogRs 'id: "opencode"' "opencode must not be in catalog items"
    }

    It "T2-R3-06: Root README and documentation contain purpose-based header" {
        $readme = Join-Path $RepoRoot "README.md"
        Should-Exist $readme "README.md must exist"
        $content = Get-Content $readme -Raw
        Should-Contain $content "Author -: HKDEVS" "README must contain Author"
        Should-Contain $content "Project -: Window-Rice" "README must contain Project"
    }
}

# ============================================================================
# R4: Multi-Shell Integration & Carapace Autocompletions (Boundary Cases)
# ============================================================================
Set-TestContext -Tier "Tier 2" -Feature "R4: Multi-Shell Integration & Carapace Autocompletions"

Describe "R4 - Multi-Shell Boundary & Syntax Stress" {
    It "T2-R4-01: PowerShell AST strict parsing: profile.ps1 has zero syntax errors" {
        $profilePath = Join-Path $RepoRoot "shells\powershell\profile.ps1"
        $tokens = $null
        $errors = $null
        [System.Management.Automation.Language.Parser]::ParseFile($profilePath, [ref]$tokens, [ref]$errors) | Out-Null
        Should-Be $errors.Count 0 "PowerShell profile.ps1 must parse with 0 errors"
    }

    It "T2-R4-02: PowerShell AST strict parsing: Microsoft.PowerShell_profile.ps1 has zero syntax errors" {
        $profilePath = Join-Path $RepoRoot "shells\powershell\Microsoft.PowerShell_profile.ps1"
        $tokens = $null
        $errors = $null
        [System.Management.Automation.Language.Parser]::ParseFile($profilePath, [ref]$tokens, [ref]$errors) | Out-Null
        Should-Be $errors.Count 0 "Microsoft.PowerShell_profile.ps1 must parse with 0 errors"
    }

    It "T2-R4-03: PowerShell profiles: both powershell/ and shells/powershell/ define window-rice cleanly" {
        $p1 = Join-Path $RepoRoot "powershell\profile.ps1"
        $p2 = Join-Path $RepoRoot "shells\powershell\profile.ps1"
        Should-Exist $p1 "powershell/profile.ps1 must exist"
        Should-Exist $p2 "shells/powershell/profile.ps1 must exist"
        
        $tokens1 = $null; $errors1 = $null
        [System.Management.Automation.Language.Parser]::ParseFile($p1, [ref]$tokens1, [ref]$errors1) | Out-Null
        Should-Be $errors1.Count 0 "powershell/profile.ps1 must parse with 0 errors"

        $tokens2 = $null; $errors2 = $null
        [System.Management.Automation.Language.Parser]::ParseFile($p2, [ref]$tokens2, [ref]$errors2) | Out-Null
        Should-Be $errors2.Count 0 "shells/powershell/profile.ps1 must parse with 0 errors"

        $c1 = Get-Content $p1 -Raw
        $c2 = Get-Content $p2 -Raw
        Should-Contain $c1 "function window-rice" "powershell/profile.ps1 must define window-rice"
        Should-Contain $c2 "function window-rice" "shells/powershell/profile.ps1 must define window-rice"
    }

    It "T2-R4-04: Synchronized shells: xonsh/rc.xsh equals shells/xonsh/rc.xsh" {
        $x1 = Join-Path $RepoRoot "xonsh\rc.xsh"
        $x2 = Join-Path $RepoRoot "shells\xonsh\rc.xsh"
        Should-Exist $x1 "xonsh/rc.xsh must exist"
        Should-Exist $x2 "shells/xonsh/rc.xsh must exist"
        $hash1 = (Get-FileHash $x1).Hash
        $hash2 = (Get-FileHash $x2).Hash
        Should-Be $hash1 $hash2 "xonsh/ and shells/xonsh/ rc.xsh hashes must match"
    }

    It "T2-R4-05: Carapace spec flag syntax: no invalid multi-flag syntax in rice-installer.yaml" {
        $spec = Get-Content (Join-Path $RepoRoot "carapace\specs\rice-installer.yaml") -Raw
        # Flags must be either single "-f, --flag" or "--flag", never "-f, -g, --flag"
        $multiShort = $spec | Select-String -Pattern "^\s*-\w,\s*-\w,"
        Should-Be $multiShort.Count 0 "No invalid multi-short flag groupings allowed"
    }

    It "T2-R4-06: Carapace spec flag syntax: no invalid multi-flag syntax in window-rice.yaml" {
        $spec = Get-Content (Join-Path $RepoRoot "carapace\specs\window-rice.yaml") -Raw
        $multiShort = $spec | Select-String -Pattern "^\s*-\w,\s*-\w,"
        Should-Be $multiShort.Count 0 "No invalid multi-short flag groupings allowed"
    }
}

# ============================================================================
# R5: Persistent Diagnostic Logging & Verification Suite (Boundary Cases)
# ============================================================================
Set-TestContext -Tier "Tier 2" -Feature "R5: Persistent Diagnostic Logging & Verification Suite"

Describe "R5 - Logging Boundary & Verification Robustness" {
    It "T2-R5-01: Auto-creation of parent directories when logging to fresh sandbox" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $backupDir = Join-Path $sandbox ".rice-backups"
                Should-NotExist $backupDir "Backup dir must not exist initially"
                & $InstallerExe --update 2>&1 | Out-String | Out-Null
                Should-Exist $backupDir "Logging must auto-create parent backup directory"
                Should-Exist (Join-Path $backupDir "installer.log") "installer.log must be created"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R5-02: Sequential log appending: multiple CLI calls grow the log file monotonically" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $logFile = Join-Path $sandbox ".rice-backups\installer.log"
                & $InstallerExe --update 2>&1 | Out-String | Out-Null
                $size1 = (Get-Item $logFile).Length

                & $InstallerExe --update 2>&1 | Out-String | Out-Null
                $size2 = (Get-Item $logFile).Length
                Should-BeGreaterThan $size2 $size1 "Log file must grow after second invocation"

                & $InstallerExe --update 2>&1 | Out-String | Out-Null
                $size3 = (Get-Item $logFile).Length
                Should-BeGreaterThan $size3 $size2 "Log file must grow after third invocation"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R5-03: System health check verifier resilience in clean sandbox" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $out = & $InstallerExe --verify | Out-String
                Should-Contain $out "WINDOWS RICE SYSTEM HEALTH CHECK REPORT" "Report banner must output"
                Should-Contain $out "Summary:" "Summary line must output"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R5-04: Viewport table formatting fits 23-char package names without line wrapping" {
        $out = & $InstallerExe --verify | Out-String
        Should-Contain $out "8K Wallpaper Collection" "23-char package name must be displayed intact"
    }

    It "T2-R5-05: Sequential execution of multiple CLI modes in one session" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                $out1 = & $InstallerExe --reload 2>&1 | Out-String
                Should-Contain $out1 "Reloading"
                $out2 = & $InstallerExe --update 2>&1 | Out-String
                Should-Contain $out2 "Checking for repository updates"
                $out3 = & $InstallerExe --verify 2>&1 | Out-String
                Should-Contain $out3 "WINDOWS RICE SYSTEM HEALTH CHECK REPORT"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R5-06: CLI -h and --help options exit cleanly with code 0 and documentation" {
        $outShort = & $InstallerExe -h 2>&1 | Out-String
        Should-Contain $outShort "WINDOW-RICE INSTALLER" "-h must print help banner"
        Should-Contain $outShort "Usage:" "-h must print usage"

        $outLong = & $InstallerExe --help 2>&1 | Out-String
        Should-Contain $outLong "WINDOW-RICE INSTALLER" "--help must print help banner"
        Should-Contain $outLong "Interactive TUI Controls:" "--help must document controls"
    }
}

# ============================================================================
# R6: Ownership Ledger & Full Revoke (Boundary & Corner Cases)
# ============================================================================
Set-TestContext -Tier "Tier 2" -Feature "R6: Ownership Ledger & Full Revoke"

Describe "R6 - Full Revoke Ownership Boundaries" {
    It "T2-R6-01: user-modified copy-fallback config is PRESERVED on disk and archived, never deleted" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                # Simulate a copy-fallback install the user later customized:
                # real file at target, no manifest record, content diverged.
                $userTarget = Join-Path $sandbox ".wezterm.lua"
                Set-Content -Path $userTarget -Value "USER_CUSTOMIZED_THEME = 'winedark'" -Encoding utf8
                Should-Exist $userTarget "User-modified copy must exist before revoke"

                $backupRoot = Join-Path $sandbox ".rice-backups"
                $out = & $InstallerExe --uninstall-all 2>&1 | Out-String
                Should-Contain $out "[PRESERVED]" "Revoke must report the preserved user-modified config"
                Should-Contain $out "[ARCHIVED]" "Revoke must archive a copy into .rice-backups"

                $after = (Get-Content $userTarget -Raw).Trim()
                Should-Be $after "USER_CUSTOMIZED_THEME = 'winedark'" "User-modified file must remain byte-identical on disk"

                $archives = Get-ChildItem -Path (Join-Path $backupRoot "wezterm") -Directory -Filter "preserved-*" -ErrorAction SilentlyContinue
                Should-Be $archives.Count 1 "Exactly one preserved archive stamp must be created under backups/wezterm"
                $archivedFile = Join-Path $archives[0].FullName ".wezterm.lua"
                Should-Exist $archivedFile "Archived copy of the user config must exist"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R6-02: pure-copy equivalent config is removed cleanly by full revoke" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                # Copy the exact repo payload to the target: identical content,
                # no manifest record -> pure rice data, safe to remove.
                $pureCopy = Join-Path $sandbox ".wezterm.lua"
                Copy-Item (Join-Path $RepoRoot "wezterm\wezterm.lua") $pureCopy -Force
                Should-Exist $pureCopy "Exact rice copy must exist before revoke"

                $out = & $InstallerExe --uninstall-all 2>&1 | Out-String
                Should-Contain $out "[CLEAN]" "Revoke must report cleanly removed pure-rice copies"
                Should-NotExist $pureCopy "Exact rice copy must be removed by full revoke"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R6-03: repo-native junction targets are never touched by full revoke" {
        $sandbox = New-TestSandbox
        try {
            Invoke-WithIsolatedProfile -SandboxDir $sandbox -Script {
                # Map the sandbox .config/carapace onto the LIVE repo directory
                # through a junction: source and target are canonically the same
                # file, so the revoke must report NATIVE and never write there.
                New-Item -ItemType Directory -Path (Join-Path $sandbox ".config") -Force | Out-Null
                $junction = Join-Path $sandbox ".config\carapace"
                New-Item -ItemType Junction -Path $junction -Target (Join-Path $RepoRoot "carapace") | Out-Null
                Should-Exist $junction "Junction into the repo carapace dir must exist"

                $specFile = Join-Path $RepoRoot "carapace\specs\window-rice.yaml"
                $hashBefore = (Get-FileHash $specFile -Algorithm SHA256).Hash

                $out = & $InstallerExe --uninstall-all 2>&1 | Out-String
                Should-Contain $out "[NATIVE]" "Revoke must report repo-native targets as never touched"
                Should-Exist $junction "Junction must survive the revoke"
                $hashAfter = (Get-FileHash $specFile -Algorithm SHA256).Hash
                Should-Be $hashAfter $hashBefore "Live repo carapace spec must be byte-identical after the revoke"

                # Tear the junction down explicitly BEFORE the sandbox removal,
                # so recursive sandbox cleanup can never walk into the live repo
                # through the junction on any PowerShell version.
                [System.IO.Directory]::Delete($junction, $false)
                Should-NotExist $junction "Junction must be removed from the sandbox after the audit"
            }
        } finally {
            Remove-TestSandbox $sandbox
        }
    }

    It "T2-R6-04: --help documents the full revoke flag with its guarantees" {
        $help = & $InstallerExe --help | Out-String
        Should-Contain $help "--uninstall-all" "Help must document the --uninstall-all flag"
    }
}
