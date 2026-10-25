# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: Windows Rice End-to-End Test Infrastructure Specification
# ============================================================================

# Windows Rice End-to-End (E2E) Test Infrastructure

## 1. Test Philosophy

The Windows Rice turnkey management ecosystem requires high-assurance verification across desktop window managers, status bar widgets, multi-shell integrations, CLI installers, and backup engines. The E2E test infrastructure adheres to three core tenets:

1. **Opaque-Box Verification**: The system is treated as a black box. Tests exercise the public observable interfaces—CLI commands, exit codes, stdout/stderr streams, file system state mutations, symlink integrity, diagnostic logs, and shell environment bindings—rather than relying on internal implementation private details.
2. **Requirement-Driven Architecture**: Every test case directly traces to the authoritative requirements defined in `ORIGINAL_REQUEST.md` (R1 through R5). No facade or dummy assertions are permitted; each assertion verifies genuine functionality.
3. **Deterministic Test Isolation**: All tests must be hermetic and reproducible. When exercising backup, symlinking, and restoration mechanisms, tests utilize temporary sandboxes (overriding `USERPROFILE` and `RICE_BACKUP_DIR`) to prevent side effects on the developer's live desktop or user profile.

---

## 2. Feature Inventory & Coverage Matrix

| Req | Functional Area | Feature Description | Tier 1 (Coverage) | Tier 2 (Boundary) | Tier 3 (Cross-Feature) | Tier 4 (Scenarios) |
|---|---|---|---|---|---|---|
| **R1** | TUI Input & Navigation | Case-insensitive keyboard shortcuts (`q/Q`, `j/J`, `k/K`, `a/A`, `n/N`, `m/M`, `l/L`, `u/U`, `x/X`, `i/I`, `r/R`, `c/C`, `w/W`, `v/V`, `b/B`, `Esc`, `Delete`) | T1-R1-01 to 05 | T2-R1-01 to 05 | T3-COMB-01, T3-COMB-03 | T4-SCEN-01 |
| **R1** | TUI Input & Navigation | Dual-key selection toggle (`Space` and `Enter`) and extended navigation (`PageUp`, `PageDown`, `Home`, `End`) | T1-R1-06 to 08 | T2-R1-06 to 08 | T3-COMB-03 | T4-SCEN-01 |
| **R2** | Non-Destructive Unlinking | Automatic backup of pre-existing user configurations to `.rice-backups/<pkg>/<timestamp>/` with `manifest.json` | T1-R2-01 to 03 | T2-R2-01 to 03 | T3-COMB-01, T3-COMB-02 | T4-SCEN-01 |
| **R2** | Non-Destructive Unlinking | Non-destructive unlinking (`--unlink`, `-x`), pristine restoration of backups, orphan directory cleanup, and binary preservation | T1-R2-04 to 07 | T2-R2-04 to 07 | T3-COMB-01, T3-COMB-02 | T4-SCEN-01, T4-SCEN-02 |
| **R3** | Attribution & Cleanliness | Purpose-based credit headers with explicit, accurate functional titles across repository configs, shaders, and scripts | T1-R3-01 to 04 | T2-R3-01 to 03 | T3-COMB-04 | T4-SCEN-03 |
| **R3** | Attribution & Cleanliness | Omission and `.gitignore` status of deprecated directories (`rammap`, `configstore`, `freebuff-desktop`, `kilo`, `opencode`) | T1-R3-05 to 06 | T2-R3-04 to 05 | T3-COMB-04 | T4-SCEN-03 |
| **R4** | Multi-Shell Integration | `window-rice` function and `rice` alias across PowerShell 7, Nushell, and Xonsh with typed flags and short aliases | T1-R4-01 to 05 | T2-R4-01 to 04 | T3-COMB-02, T3-COMB-05 | T4-SCEN-04, T4-SCEN-05 |
| **R4** | Shells & Carapace | Carapace autocompletion specifications (`rice-installer.yaml`, `window-rice.yaml`) validation without invalid multi-flag syntax | T1-R4-06 to 07 | T2-R4-05 to 06 | T3-COMB-05 | T4-SCEN-05 |
| **R5** | Logging & Verification | Structured timestamped diagnostic logging to `installer.log` (ISO-8601 UTC / epoch timestamps, file mutex sync) | T1-R5-01 to 03 | T2-R5-01 to 03 | T3-COMB-01, T3-COMB-05 | T4-SCEN-01, T4-SCEN-02 |
| **R5** | Logging & Verification | System health verification check (`--verify`, `-v`) reporting package managers, configs, and autocompletion status | T1-R5-04 to 06 | T2-R5-04 to 05 | T3-COMB-05 | T4-SCEN-02 |

---

## 3. 4-Tier Test Methodology

The E2E test suite is organized into 4 hierarchical validation tiers:

### Tier 1: Feature Coverage (Isolation / Happy Path)
Focuses on individual feature contracts in isolation under standard operating conditions:
- **R1**: Verifies all 17 documented hotkeys, dual-key toggle (`Space` and `Enter`), and list navigation keys through unit/CLI interfaces.
- **R2**: Verifies backup record generation, `manifest.json` schema, target file replacement, and pristine backup restoration.
- **R3**: Verifies header presence (`Author -:`, `Project -:`, `Title -:`) across config directories, and ensures removed packages are excluded from the catalog.
- **R4**: Verifies PowerShell AST parsing of profiles, Nushell function definition, Xonsh alias registration, and shell directory synchronization.
- **R5**: Verifies diagnostic log file creation, log entry formatting, CLI `--verify` output sections, and Cargo unit test execution.

*Minimum requirement*: ≥5 test cases per feature area (total ≥25 tests).

### Tier 2: Boundary & Corner Cases (Limits, Edge Cases & Error Conditions)
Stresses the system under boundary conditions, malformed inputs, and anomalous environments:
- **R1**: Navigation clamping at list extremes (50+ repeated `PageDown`/`End`/`Down` keypresses), behavior on empty item lists, rapid toggle sequences.
- **R2**: Non-existent target unlinking (`None == None` canonicalization bug prevention), read-only target files, deeply nested directory structures, unlinking when no backup exists.
- **R3**: Verifies zero occurrences of generic directory paths (e.g. `kew / kew.ini`) or template placeholders (`<respective file title>`), and verifies case variants in `.gitignore`.
- **R4**: Malformed CLI argument parsing, missing argument values (e.g. `--wallpapers` with missing path), invalid flag combinations, PowerShell AST syntax strictness (`${dstPath}:`).
- **R5**: Log writing to non-existent directories (auto-creation of parent paths), concurrent logging stress, CLI `--verify` behavior when binaries or configs are absent.

*Minimum requirement*: ≥5 test cases per feature area (total ≥25 tests).

### Tier 3: Cross-Feature Combinations (Pairwise Interoperability)
Evaluates interactions between disparate system subsystems:
- **T3-COMB-01**: **Backup -> Link -> Unlink -> Restore Cycle**: Pre-existing config backup, followed by new config linking, followed by unlinking, verifying original file restoration and removal of all orphan backup directories.
- **T3-COMB-02**: **Multi-Shell Invocation -> CLI Engine**: Testing `window-rice -x` or `-v` flag propagation from PowerShell/Nushell script blocks directly into `rice-installer.exe`.
- **T3-COMB-03**: **Interactive Navigation -> Batch Selection State**: Verifying that navigation bounds clamping correctly maintains selection checkbox states across full-list traversal.
- **T3-COMB-04**: **Catalog Exclusions -> File System Attribution**: Ensuring that excluded packages have no remaining config references or un-attributed orphan files in the active repo.
- **T3-COMB-05**: **CLI Verification -> Diagnostic Log Synchronization**: Running CLI `--verify` and asserting that the diagnostic log file synchronously records the verification audit event with an ISO-8601/timestamped entry.

### Tier 4: Real-World Application Scenarios (Turnkey End-to-End Workflows)
End-to-end simulations of actual user and system workflows:
- **T4-SCEN-01**: **Complete Rice Bootstrap & Teardown**: Simulated installation from clean user state, user config backup, rice linking, verification audit, and complete unlinking with pristine user profile restoration.
- **T4-SCEN-02**: **Non-Destructive Binary Preservation Audit**: End-to-end installation and complete unlinking cycle asserting that mocked binaries in PATH remain completely untouched and no package manager uninstall commands are triggered.
- **T4-SCEN-03**: **Repository-Wide Attribution & Git Cleanliness Audit**: Automated scanning of all tracked files in the repository ensuring strict compliance with functional purpose-based titles and clean git status.
- **T4-SCEN-04**: **Full Multi-Shell Parity Validation**: Executing and validating profile loading and function declarations across PowerShell 7 (`Microsoft.PowerShell_profile.ps1`), Nushell (`config.nu`), and Xonsh (`rc.xsh`).
- **T4-SCEN-05**: **Carapace Specification Codegen & Completion Query**: Validating both `rice-installer.yaml` and `window-rice.yaml` against Carapace schema using `carapace --codegen` and flag parsing.

---

## 4. Test Harness & Runner Architecture

The test suite is built natively in PowerShell 7 (`pwsh`), ensuring zero external dependencies, seamless execution on Windows, and direct interoperability with Windows Terminal, GlazeWM, and Scoop/Winget environments:

```
tests/e2e/
├── Common.psm1                     # Assertion engine, reporting, mocking & sandbox utilities
├── tier1_feature_coverage.Tests.ps1 # Tier 1: Isolated feature coverage (≥25 tests)
├── tier2_boundary_corner.Tests.ps1  # Tier 2: Boundary & edge cases (≥25 tests)
├── tier3_cross_feature.Tests.ps1   # Tier 3: Pairwise combinations (≥5 tests)
├── tier4_real_world.Tests.ps1      # Tier 4: Real-world user workflows (≥5 tests)
└── run_all.ps1                     # Master orchestrator with exit codes & structured summaries
```

### Key Capabilities of `Common.psm1`
- **Assertion DSL**: `Should-Be`, `Should-NotBe`, `Should-Exist`, `Should-NotExist`, `Should-Match`, `Should-Contain`, `Should-BeGreaterThan`.
- **Sandboxing**: `New-TestSandbox`, `Remove-TestSandbox`, `With-IsolatedProfile` for hermetic test execution without modifying live configs.
- **Reporting**: Colorized terminal output, pass/fail counters, timing metrics, and structured failure diagnostics.
- **Exit Code**: Returns exit code `0` on 100% pass, non-zero on any failure.

---

## 5. Execution Guide

### Run All Tiers
```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File tests/e2e/run_all.ps1
```

### Run a Specific Tier
```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File tests/e2e/tier1_feature_coverage.Tests.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File tests/e2e/tier2_boundary_corner.Tests.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File tests/e2e/tier3_cross_feature.Tests.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File tests/e2e/tier4_real_world.Tests.ps1
```

### Verbose Mode
```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File tests/e2e/run_all.ps1 -Verbose
```
