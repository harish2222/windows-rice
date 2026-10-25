# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Windows Rice End-to-End Test Readiness Specification
# ============================================================================

# Windows Rice E2E Test Suite Readiness & Verification Report

## 1. Master Test Runner Command

The complete 4-tier End-to-End test suite can be executed with a single command from PowerShell 7:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\run_all.ps1
```

### Targeted Tier Execution Commands
```powershell
# Run Tier 1: Feature Coverage Only (31 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier1_feature_coverage.Tests.ps1

# Run Tier 2: Boundary & Corner Cases Only (30 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier2_boundary_corner.Tests.ps1

# Run Tier 3: Cross-Feature Combinations Only (6 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier3_cross_feature.Tests.ps1

# Run Tier 4: Real-World Scenarios Only (6 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier4_real_world.Tests.ps1
```

---

## 2. Test Execution Summary

```
================================================================================
                           E2E TEST EXECUTION SUMMARY                           
================================================================================
Tier Breakdown:
Tier               Total   Passed   Failed  Pass Rate
-----------------------------------------------------
Tier 1                31       31        0       100%
Tier 2                30       30        0       100%
Tier 3                 6        6        0       100%
Tier 4                 6        6        0       100%

Feature Breakdown:
Feature Area                                          Total   Passed   Failed
------------------------------------------------------------------------------
R1: TUI Input Handling & Navigation                      12       12        0
R2: Non-Destructive Config Unlinking & Backup Restoration       12       12        0
R3: Functional Purpose-Based Credit Attribution          12       12        0
R4: Multi-Shell Integration & Carapace Autocompletions       13       13        0
R5: Persistent Diagnostic Logging & Verification Suite       12       12        0
Cross-Feature Integration                                 6        6        0
Real-World Application Scenarios                          6        6        0
================================================================================
TOTAL TESTS : 73
PASSED      : 73
FAILED      : 0
ELAPSED     : 59.86s
================================================================================

E2E SUITE STATUS: ALL TESTS PASSED [100% VERIFIED]
```

---

## 3. Requirement Verification & Acceptance Criteria Checklist

### R1. TUI Input Handling & Extended Navigation
- [x] **Case-Insensitive Hotkeys**: All 17 shortcuts (`q/Q`, `j/J`, `k/K`, `a/A`, `n/N`, `m/M`, `l/L`, `u/U`, `x/X`, `i/I`, `r/R`, `c/C`, `w/W`, `v/V`, `b/B`, `Esc`, `Delete`) verified via `rice-installer.exe --help` and decoupled unit tests (`test_case_insensitive_hotkeys`).
- [x] **Extended Navigation**: `PageUp`, `PageDown`, `Home`, `End`, `Up`, `Down`, `j`, `k` verified with safe boundary clamping across full list traversal.
- [x] **Dual-Key Selection Toggle**: Both `Space` (Char ' ') and `Enter` independently toggle selection checkboxes with deterministic parity.
- [x] **Extreme Bounds Resilience**: Zero-item, single-item, and 50+ repeated scroll cycles verified without panic or index underflow.
- [x] **Column & Viewport Polishing**: Verifier table and TUI accommodate 23-character package names (`8K Wallpaper Collection`) cleanly without line wrapping.

### R2. Non-Destructive Config Unlinking & Backup Restoration
- [x] **Isolated User Backup**: Pre-existing user configs are backed up to `%USERPROFILE%\.rice-backups\<pkg>\<timestamp>\` and cataloged in `manifest.json`.
- [x] **Pristine Restoration**: Running uninstallation (`--unlink`, `-x`, `--unlink-all`) cleanly removes symlinks/copies and restores original files byte-for-byte.
- [x] **Clean Restoration Without Orphan Traces**: Restored backup directories and timestamp folders are cleaned up upon successful restoration.
- [x] **Binary Preservation Guarantee**: Mock application binaries in PATH and package manager directories (`Scoop`, `Winget`, `Chocolatey`, `Cargo`) remain 100% untouched during unlinking cycles.
- [x] **Safe Canonicalization**: Target unlinking when target does not exist returns clean no-op status without spurious canonicalization errors.
- [x] **Hermetic Test Isolation**: Test suite sets isolated sandbox environment variables (`USERPROFILE`, `RICE_BACKUP_DIR`, `HOME`) ensuring zero production desktop pollution.

### R3. Functional Purpose-Based Credit Attribution & Cleanliness
- [x] **Consistent Header Structure**: Every configuration, shader, script, and documentation file contains standard headers (`Author -: HKDEVS`, `Project -: Window-Rice`).
- [x] **Purpose-Based Titles**: Explicit, functional titles (e.g. `Win32 GDI Device Context and Bitmap Interop Adapter`, `CAVA Linear Frequency Bar Spectrum Shader`, `Kew Catppuccin Mocha Soft Pastel Terminal Theme`).
- [x] **Zero Placeholder Strings**: Verified 0 occurrences of `<respective file title>` across all tracked repository files.
- [x] **Zero Generic Titles**: Verified 0 occurrences of generic `folder / name` titles across all tracked repository files.
- [x] **Catalog Exclusions**: Deprecated packages (`rammap`, `configstore`, `freebuff-desktop`, `kilo`, `opencode`, `forgum`, `nvim`, `neovim`) are omitted from active package catalog.
- [x] **Git Cleanliness**: Deprecated directories (`rammap/`, `RAMMap/`, `configstore/`, `freebuff-desktop/`, `kilo/`, `opencode/`) explicitly ignored in `.gitignore`.

### R4. Multi-Shell Integration & Carapace Autocompletions
- [x] **PowerShell Profile**: `function window-rice` defined in `profile.ps1` and `Microsoft.PowerShell_profile.ps1` with parameter aliases (`-v`, `-x`, `-c`, `-r`, `-w`, `-b`, `$WallpaperDir`).
- [x] **PowerShell Alias**: `Set-Alias -Name rice -Value window-rice -Force` verified.
- [x] **Nushell Integration**: `def --env window-rice` defined in `config.nu` with flags (`-c`, `-u`, `-x`, `-r`, `-v`, `-w`, `-b`) and alias `rice = window-rice`.
- [x] **Xonsh Integration**: `def window_rice` defined in `rc.xsh` with aliases `window-rice` and `rice`.
- [x] **Carapace Spec Codegen**: `carapace/specs/rice-installer.yaml` and `window-rice.yaml` compile cleanly to Go root packages via `carapace --codegen`.
- [x] **PowerShell AST Strictness**: `install.ps1` parses with 0 AST errors (including `${dstPath}:` string interpolation).

### R5. Persistent Diagnostic Logging & Verification Suite
- [x] **Diagnostic Logging**: Installer operations automatically append structured entries to `%USERPROFILE%\.rice-backups\installer.log`.
- [x] **Timestamp Format**: Log entries formatted with structured timestamp prefixes `[<timestamp>] <message>`.
- [x] **System Health Check**: CLI `--verify` outputs comprehensive system health report listing 8 package managers, 23 components, and 63 Carapace autocompletion specs.
- [x] **Full Unit Test Execution**:
  * `cargo test --manifest-path installer/Cargo.toml`: 10 passed, 0 failed.
  * `cargo test --manifest-path yasb/Cargo.toml`: 100+ passed across 9 crates, 0 failed.

---

## 4. Test Suite Artifacts Inventory

| File Path | Description | Test Count |
|---|---|---|
| `tests/e2e/Common.psm1` | Core test harness module, assertion DSL, sandboxing, timing & reporting | N/A |
| `tests/e2e/tier1_feature_coverage.Tests.ps1` | Tier 1: Isolated happy-path feature coverage across R1 through R5 | 31 |
| `tests/e2e/tier2_boundary_corner.Tests.ps1` | Tier 2: Boundary conditions, extremes, limits, error paths & stress | 30 |
| `tests/e2e/tier3_cross_feature.Tests.ps1` | Tier 3: Cross-feature pairwise interactions & integration workflows | 6 |
| `tests/e2e/tier4_real_world.Tests.ps1` | Tier 4: Real-world turnkey user scenarios & repository-wide audits | 6 |
| `tests/e2e/run_all.ps1` | Master test runner orchestrating all tiers with structured exit codes | 73 (total) |
| `TEST_INFRA.md` | Formal specification of test philosophy, coverage matrix & methodology | N/A |
| `TEST_READY.md` | Test execution commands, summary tables, and acceptance criteria verification | N/A |
