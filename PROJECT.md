# Project: Windows Rice Turnkey Management Ecosystem Validation, Packaging & Deployment

## Architecture
- **TUI & Core Engine (`installer/`)**: Rust application using Ratatui 0.29 + Crossterm 0.28. Manages package catalog, inspection, symlinking, copying, backup creation, and restoration.
- **YASB Status Bar Suite (`yasb/`)**: Modular multi-crate Rust workspace (9 crates: `keybind`, `komorebi_sync`, `palette_picker`, `pomodoro`, `saka`, `saka_popup`, `yasb_theme`, `yasb_chrome`, `screen-shot`) providing Win32 layered acrylic paneling and desktop widgets.
- **Multi-Shell Integration Layer (`shells/`, `powershell/`, `xonsh/`)**: Advanced shell functions (`window-rice` / `rice`) across PowerShell 7, Nushell, and Xonsh with real-time status query, unlinking, updating, and environment reloading.
- **Autocompletions (`carapace/specs/`)**: 63 Carapace command completion schemas for CLI tools and rice management functions.
- **Installation & Setup Scripts (`install.ps1`)**: Windows turnkey bootstrapping script for Scoop, Chocolatey, UV, Vcpkg, and application packages.
- **Continuous Integration (`.github/workflows/`)**: GitHub Actions workflows `ci.yml` and `release.yml` executing syntax validation, unit tests, and build artifacts.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Case-Insensitive TUI Hotkeys | Support all 15 hotkeys (`q/Q`, `j/J`, `k/K`, `a/A`, `n/N`, `m/M`, `l/L`, `u/U`, `x/X`, `i/I`, `r/R`, `c/C`, `w/W`, `v/V`, `b/B`, `Esc`, `Delete`) | M1 | R1, survey_1 |
| 2 | Extended Keyboard Navigation | Support `PageUp`, `PageDown`, `Home`, `End`, `Up`, `Down`, `j`, `k` with safe bounds | M1 | R1, survey_1 |
| 3 | Dual-Key Package Selection | Toggle package selection checkbox with both `Space` and `Enter` | M1 | R1, survey_1 |
| 4 | Decoupled Key Event Handling | Extract `App::handle_key` from `run_tui()` for automated unit testing | M1 | R1, survey_1 |
| 5 | TUI Column & Viewport Polishing | Fix verifier column width for 23-char package names and support terminal bounds | M1 | R1, survey_1 |
| 6 | Unit Test Expansion for TUI | Automated unit tests for hotkeys, bounds, and dual-key selection in `installer` | M1 | R1, R5, survey_1 |
| 7 | CLI `--unlink` / `-x` Package Selection | Fix CLI unlink to process all linked configs when no explicit package arguments passed | M2 | R2, survey_2 |
| 8 | Safe Backup Restoration Canonicalization | Fix `None == None` match bug in `linker::restore_target_backup` when target does not exist | M2 | R2, survey_2 |
| 9 | Clean Restoration Without Orphan Traces | Delete empty timestamp and package folders under `%USERPROFILE%\.rice-backups\` on restore | M2 | R2, survey_2 |
| 10 | Non-Destructive Binary Preservation | Ensure unlinking never touches binaries in PATH and never invokes package managers | M2 | R2, survey_2 |
| 11 | Test Isolation with `RICE_BACKUP_DIR` | Support environment override in `get_backup_root()` to prevent real profile pollution | M2 | R2, R5, survey_2 |
| 12 | Diagnostic Logging Synchronization & ISO-8601 | File-lock / mutex synchronization on `installer.log` with ISO-8601 UTC timestamps | M2 | R5, survey_2 |
| 13 | CLI Verification Logging | Ensure CLI `--verify` appends audit results to `installer.log` | M2 | R5, survey_1, survey_2 |
| 14 | Purpose-Based Attribution Headers | Replace all 86 generic titles (CAVA, CLIamp, Kew, Otty, READMEs) with functional titles | M3 | R3, survey_2 |
| 15 | Repository Cleanliness & Excluded Directories | Verify omission and `.gitignore` status of `rammap`, `configstore`, `freebuff-desktop`, `kilo`, `opencode` | M3 | R3, survey_2 |
| 16 | Fatal PowerShell AST Syntax Fix in `install.ps1` | Replace `$dstPath:` with `${dstPath}:` on line 223 to fix parser error | M4 | R4, survey_3 |
| 17 | PowerShell `window-rice` Parameter Aliases | Add `[Alias('x')]`, `[Alias('c')]`, `[Alias('r')]`, `[Alias('v')]`, `[Alias('w')]`, `[Alias('b')]`, `$WallpaperDir` | M4 | R4, survey_3 |
| 18 | Nushell `window-rice` Flag Alignment | Bind `-c` to `--update` and `-u` to `--check-updates` in `shells/nushell/config.nu` | M4 | R4, survey_3 |
| 19 | Shell Directory Synchronization | Synchronize root `powershell/` and `xonsh/` with `shells/powershell/` and `shells/xonsh/` | M4 | R4, survey_3 |
| 20 | Carapace Completion Specs Fix | Split invalid multi-flag syntax in `rice-installer.yaml` and `window-rice.yaml` | M4 | R4, survey_3 |
| 21 | YASB Workspace CI Integration | Add Windows runner step in `.github/workflows/ci.yml` for `cargo test --manifest-path yasb/Cargo.toml` | M4 | AC, survey_3 |
| 22 | Strict Carapace CI Validation | Upgrade Carapace check in `ci.yml` to run `carapace --codegen` or schema check | M4 | AC, survey_3 |
| 23 | E2E Testing Suite (Tiers 1-4) | Comprehensive opaque-box test suite for R1-R5 requirements | E2E Track | ORIGINAL_REQUEST |
| 24 | Final Milestone E2E Pass & Hardening | Execute 100% E2E tests, Tier 5 adversarial hardening, and validation | M5 | AC, Project Pattern |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | TUI Input Handling & Navigation | Refactor `App::handle_key`, test all hotkeys, navigation bounds, dual-key toggle, and verifier formatting | none | PLANNED |
| M2 | Unlink Engine, Backup Restoration & Logging | Fix CLI unlink selection, restore canonicalization, orphan directory cleanup, test isolation, logging mutex, ISO-8601 | none | PLANNED |
| M3 | Purpose-Based Credit Attribution & Cleanliness | Update 86 files with functional purpose-based titles; verify excluded directories | none | PLANNED |
| M4 | Multi-Shell Integration & Carapace Specs | Fix `install.ps1:223`, PowerShell/Nushell/Xonsh `window-rice` flags, Carapace specs, CI/CD | none | PLANNED |
| E2E | E2E Testing Track | Requirement-driven opaque-box test infrastructure and Tiers 1-4 test cases | none | IN_PROGRESS |
| M5 | Final Milestone E2E Pass & Hardening | Pass 100% E2E test suite, adversarial coverage hardening (Tier 5), full test execution | M1, M2, M3, M4, E2E | PLANNED |

## Interface Contracts
### `installer::app` ↔ `installer::main`
- `App::handle_key(&mut self, code: ratatui::crossterm::event::KeyCode) -> bool`: returns true if should quit, handles all hotkeys and navigation keys.
### `installer::linker` ↔ Environment
- `get_backup_root() -> PathBuf`: respects `RICE_BACKUP_DIR` environment variable override if set; otherwise falls back to `%USERPROFILE%\.rice-backups\`.
- `restore_target_backup(package_id: &str, target: &Path) -> Result<Option<PathBuf>>`: safely compares target canonicalization only if `is_ok()`, removes target symlink/copy, restores original files, and prunes empty timestamp/package directories.
### Shell CLI ↔ `rice-installer.exe`
- Flags: `--unlink` (`-x`), `--update` (`-c`), `--reload` (`-r`), `--verify` (`-v`), `--wallpapers` (`-w`), `--bootstrap` (`-b`).
- Carapace spec: each flag entry contains at most one short flag and one long flag.

## Code Layout
- `installer/src/main.rs`: Entry point, CLI argument parsing, TUI event loop setup.
- `installer/src/app.rs`: Application state, key handling logic, navigation, action dispatch, logging.
- `installer/src/linker.rs`: Symlink/copy operations, backup creation, manifest handling, restoration, backup directory management.
- `installer/src/verifier.rs`: System health report generation and table formatting.
- `install.ps1`: PowerShell installation script.
- `shells/powershell/` & `powershell/`: PowerShell profile scripts and `window-rice` function.
- `shells/nushell/`: Nushell `config.nu` and `window-rice` function.
- `shells/xonsh/` & `xonsh/`: Xonsh `rc.xsh` / `.xonshrc` and `window_rice` function.
- `carapace/specs/`: Carapace autocompletion specification files.
- `.github/workflows/`: CI/CD workflow YAML definitions.
- `cava/shaders/`, `cliamp/`, `kew/`, `otty/`: Rice configurations requiring purpose-based attribution headers.
- `tests/e2e/`: Opaque-box E2E test suite managed by E2E Testing Track.
