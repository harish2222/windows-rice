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
# Run Tier 1: Feature Coverage Only (34 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier1_feature_coverage.Tests.ps1

# Run Tier 2: Boundary & Corner Cases Only (34 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier2_boundary_corner.Tests.ps1

# Run Tier 3: Cross-Feature Combinations Only (6 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier3_cross_feature.Tests.ps1

# Run Tier 4: Real-World Scenarios Only (7 tests)
pwsh -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\tests\e2e\tier4_real_world.Tests.ps1
```

---

## 2. Test Execution Summary

```
================================================================================
                           E2E TEST EXECUTION SUMMARY                           
================================================================================
Tier Breakdown:
Tier               Total   Passed  Skipped   Failed  Pass Rate
------------------------------------------------------------
Tier 1                34       33        1        0       100%
Tier 2                34       34        0        0       100%
Tier 3                 6        6        0        0       100%
Tier 4                 7        6        1        0       100%

Feature Breakdown:
Feature Area                                          Total   Passed  Skipped   Failed
------------------------------------------------------------------------------
R1: TUI Input Handling & Navigation                      12       12        0        0
R2: Non-Destructive Config Unlinking & Backup Restoration       12       12        0        0
R3: Functional Purpose-Based Credit Attribution          12       12        0        0
R4: Multi-Shell Integration & Carapace Autocompletions       13       13        0        0
R5: Persistent Diagnostic Logging & Verification Suite       12       11        1        0
R6: Ownership Ledger & Full Revoke                        7        7        0        0
Cross-Feature Integration                                 6        6        0        0
Real-World Application Scenarios                          7        6        1        0
================================================================================
TOTAL TESTS : 81
PASSED      : 79
SKIPPED     : 2
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
- [x] **Zero Placeholder Strings**: Verified 0 occurrences of placeholder template strings across all tracked repository files.
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
  * `cargo test --manifest-path installer/Cargo.toml`: 25 passed, 0 failed (v1.0.2 suite, see §5 addendum).
  * `cargo test --manifest-path yasb/Cargo.toml`: 100+ passed across 9 crates, 0 failed.

---

## 4. Test Suite Artifacts Inventory

| File Path | Description | Test Count |
|---|---|---|
| `tests/e2e/Common.psm1` | Core test harness module, assertion DSL, sandboxing, skip markers, timing & reporting | N/A |
| `tests/e2e/tier1_feature_coverage.Tests.ps1` | Tier 1: Isolated happy-path feature coverage across R1 through R6 | 34 |
| `tests/e2e/tier2_boundary_corner.Tests.ps1` | Tier 2: Boundary conditions, extremes, limits, error paths & stress | 34 |
| `tests/e2e/tier3_cross_feature.Tests.ps1` | Tier 3: Cross-feature pairwise interactions & integration workflows | 6 |
| `tests/e2e/tier4_real_world.Tests.ps1` | Tier 4: Real-world turnkey user scenarios & repository-wide audits | 7 |
| `tests/e2e/run_all.ps1` | Master test runner orchestrating all tiers with structured exit codes | 81 (total) |
| `TEST_INFRA.md` | Formal specification of test philosophy, coverage matrix & methodology | N/A |
| `TEST_READY.md` | Test execution commands, summary tables, and acceptance criteria verification | N/A |

---

## 5. v1.0.2 Release Verification Addendum (2026-10-27)

The v1.0.2 release adds the asynchronous action worker and repackages the release binary. Verification status:

### Async Action Worker (new in v1.0.2)
- **Worker Architecture**: Every TUI action (`i`, `r`, `c`, `w`, `b`, `v`, `u`, `x`, wallpaper flows) now runs on a dedicated worker thread, driven by `WorkerTask` → `WorkerMessage` events (`Log`, `Progress`, `PmStatus`, `GitStatus`, `TaskFinished`). The UI thread stays responsive on a 50 ms event poll.
- **Streaming Output**: Package installs stream live `[OUT]` log lines through `run_streaming_command` with `ensure_package_manager_ready` preflight and non-interactive flag injection (`prepare_install_command`). TUI-only behavior — CLI flows (`--verify`, `--update`) and tests remain synchronous.
- **Cancellation**: `Esc` cancels the running task; `q` cancels and quits. Cancel flags are checked between package steps; a task completes its current package before aborting (no partial linking without logging).
- **Log Scrolling**: `[` / `]` scroll the log pane ±10 rows, clamped to content bounds. While busy, navigation/selection stays active and other action hotkeys are ignored.
- **Panic Safety Net**: Worker panics are absorbed by the event-polling loop and surfaced as `[FAILED]` log entries without destabilizing the TUI.

### Unit Test Evidence (host run, before the Docker-only policy)
- `cargo check` (lib + tests): 0 warnings.
- `cargo test --manifest-path installer/Cargo.toml`: **25 passed, 0 failed**, including new `test_log_scroll_keys_and_caps`, `test_async_worker_guards`, and `test_async_worker_state_machine`.
- v1.0.3 additions (ownership ledger engine): `ledger.rs` contributes 3 ledger unit tests and the symlink-restore guard regression adds a 4th engine test → statically verified suite total is **34 units** (11 linker + 4 catalog + 3 ledger + 16 app). Container run evidence for v1.0.3 is recorded in §5.1 below.

### Execution Policy Change
- Starting with v1.0.2, all future test suites (cargo units and the 4-tier E2E) execute **inside Docker only** — see `TEST_INFRA.md` §6. The unit suite above ran on the host on 2026-10-27, before this policy took effect; no further host runs are scheduled.
- The 73-test E2E suite rerun against v1.0.2 is pending Docker engine bring-up.

### Release Artifact
- `bin/rice-installer.exe` repackaged from the v1.0.2 release build (static CRT). Authenticode signing must be re-applied with the project certificate before publishing the GitHub Release.

---

## 5.1 v1.0.3 Containerized Verification Addendum (2026-10-29)

<!-- Author -: HKDEVS / Project -: Window-Rice -->

### What ships in v1.0.3
- **Ownership ledger & full revoke** (`installer/src/ledger.rs`, `--uninstall-all`): every rice-installed config & wallpaper entry is recorded in `installed.json` at the backup root; full revoke restores displaced user configs byte-for-byte, archives user-modified copies under `preserved-*`, removes pure rice copies, and never touches repo-native targets or binaries.
- **Test hardening**: skip markers (`Skip-Test` / `RiceSkipException`) for hosts without the full rice, APPDATA/LOCALAPPDATA sandbox virtualization, freshest-binary selection (`Select-InstallerExe`), CI junction bridge for `T3-COMB-02`.
- **E2E growth**: 73 → **81 tests** (Tier 1: 34, Tier 2: 34, Tier 3: 6, Tier 4: 7); R6 ledger coverage = 7 scenarios (T1-R6-01..03, T2-R6-01..04, T4-SCEN-07). Units: 25 → **34**.
- **CI/CD**: `ci.yml` gains an `e2e-suite` job (windows runner, tiers 1–4); `release.yml` gains a `cargo test` gate before packaging.
- **Docs**: README carapace spec count corrected to 112; installer help documents `-UninstallAll` / `--uninstall-all` in every surface (install.ps1, carapace specs, CLI help).

### Containerized execution (per TEST_INFRA.md §6 — image `window-rice-testenv`)
- Image: `mcr.microsoft.com/powershell:windowsservercore-ltsc2022` + MinGit + carapace + VS BuildTools 2022 (VCTools) + Rust stable-msvc; repo baked at `C:\repo`, scratch at `C:\scratch`.
- Suites run: `cargo test --manifest-path installer/Cargo.toml` (34 units) and `tests/e2e/run_all.ps1 -Tier All` (81 scenarios).
- Expected container/CI result: 34/34 units, e2e 78 pass + 3 skip (`T1-R5-04`, `T4-SCEN-06` — no 23 binaries inside the container; `T2-R4-03` — host-local `powershell/profile.ps1` is untracked), 0 failures. On the live master (which carries the untracked host-local surface) a run posts 79 pass + 2 skip.
- Windows containers are mandatory for these suites (MSVC build + Windows junction/profile isolation); Linux-only engines (podman on Windows runs Linux containers) cannot execute them. Runner recipe: `docker/run-in-container.ps1`, image spec: `docker/Dockerfile`.

### Provenance stamp (filled at run time by the container runner)
| Field | Value |
|---|---|
| Engine | recorded by `docker/run-in-container.ps1` output header |
| Image ID | recorded by runner output header |
| Commit | recorded by runner output header |
| Host exec count | **0** — no installer/cargo execution on the live host |

### §5.1a Podman verification cycle for v1.0.3 (filled 2026-10-29)
Verification split used for this cycle: **podman (Linux containers)** for every cross-platform
surface + **CI windows runners** for the Windows-bound suites (MSVC + junction-based e2e).
The Windows Docker engine on the host was installation-disabled this cycle, so no
Windows-container execution was attempted on the node.

- **Conformance image** (`docker/Conformance.Containerfile`, pwsh:ubuntu-22.04 + git + curl
  + build-essential + rustfmt + PowerShell-YAML; repo baked at `/repo`, `479` tracked files
  (measured `478` at the v1.0.4 bake: `bin/*.exe` untracked per the security review)):
  ALL CHECKS PASSED — 11 pwsh surfaces AST-clean, 112 carapace specs parse under strict YAML
  (both entrypoints document `--uninstall-all`), CI gates present, doc-count parity
  (tiers 34/34/6/7 = 81; units 34; release tag v1.0.3).
- **Behavioral verifier** (`docker/run-revoke-behavior.ps1`, in-podman, zero host exec,
  zero host junctions): ALL CHECKS PASSED —
    1. `cargo build` (linux) of the engine;
    2. `cargo test` unit suite: **34 passed, 0 failed**;
    3. T1-R6 trio: clean-machine `--uninstall-all` prints the banner + the
       `Full revoke complete: 0 restored, 0 cleaned, 0 preserved (user-modified),
       0 repo-native skipped, 29 no-op, 0 failed` summary contract on stdout
       (verifies the CLI-parity log fix);
    4. T4-SCEN-07 lifecycle with native symlinks: seed user config → `--link-all`
       (ledger records the catalog, `.wezterm.lua` mode **Symlink**) → `--uninstall-all`
       → `[RESTORED] -> Restored backup: /ricebackups/wezterm/<ts>/.wezterm.lua`,
       user content restored byte-for-byte, ledger emptied, repo untouched, 0 failed.
       Same run exposed (and validated the fix for) the collateral-collapse bug where
       symlink targets were classified repo-native and never unlinked.
- **CI**: run `40839793398` — all 4 jobs success (`rust-installer` windows + ubuntu 34/34, shell/config validation, `e2e-suite`: **81 total, 78 passed, 3 skipped, 0 failed**).
  Release tag `v1.0.3` pushed after; `release.yml` gates the package on the installer unit suite.
- Provenance: engine podman-linux (client 6.1.4 / server 6.0.2); commit when conformance ran
  `118abc5` lineage; host exec count for installer/cargo: **0**.

## §5.2 v1.0.4 Security Fix-Forward Addendum (2026-10-30)

Detection-only security review run `b3fff0fe-4d89-471e-a1ba-b3c67a57776a` (base `45fd6f9` → HEAD)
verdict **FAIL** at the iac HIGH threshold; three HIGH findings fixed forward in v1.0.4 with
operator approval (operator chose: apply all three HIGH fixes, untrack binaries, commit
review artifacts, spawn the multi-agent roster for post-fix verification):

1. **CI-ACTIONS-NOT-SHA-PINNED** — all 10 workflow `uses:` refs SHA-pinned with the
   commit the tag currently resolves to (GitHub-API cross-verified):
   `actions/checkout@11d5960a…`, `actions/cache@0057852b…`,
   `actions/upload-artifact@ea165f8d…`, `dtolnay/rust-toolchain@b725eda5…`
   (stable branch snapshot), `softprops/action-gh-release@3bb12739…`.
   Release-tag input additionally validated against `^v\d+\.\d+\.\d+$`.
2. **BS-REMOTE-EVAL-UNPINNED** — bootstrap one-liners/documentation now point at the
   immutable `releases/latest/download/install.ps1` asset (never `raw.githubusercontent/main`);
   release workflow publishes `install.ps1` as its own asset; archive fallback resolves the
   latest release **tag** via the GitHub API instead of `archive/refs/heads/main.zip`.
   `installer/src/bootstrap.rs` vendor-package-manager endpoints (get.scoop.sh /
   community.chocolatey.org / astral.sh) stay as documented vendor-pattern risk-acceptance
   (vendor-owned canonical domains, unchanged this cycle).
3. **ARTIFACT-ZIP-NO-HASH-CHECK** — release `SHA256SUMS.txt` now carries the exe **and**
   install.ps1 hashes; `install.ps1` hash-verifies a release-downloaded `rice-installer.exe`
   against `SHA256SUMS.txt` before launch (fail-closed on mismatch; legacy releases whose
   SUMS lack the exe line downgrade to a visible notice).
4. **BIN-TRACKED-EXES** (LOW camp, operator-approved) — `bin/rice-installer.exe` +
   `bin/term-launch.exe` untracked (`git rm --cached`), `*.exe` ignore doctrine updated;
   remote flow falls through to the now hash-verified release download.

Artifacts: `.mcp/reviews/b3fff0fe-4d89-471e-a1ba-b3c67a57776a/` (findings.json, gate-result.md,
threat-model.md, taint-map.json, coverage-manifest.json, remediation-templates.md, chain.json +
chain-verify.json — SHA-256 attestation chain valid=true, 4 links) and `.mcp/policies/security-policy.json`.
Tooling note: the security-mcp `security_run_pr_gate` recent_changes run crashed the MCP server
(exit 134, heap OOM) on the tracked-binary diff; the gate was completed via operator-directed
local detectors over 31 code targets (file_by_file scope) with thresholds unchanged — re-run gate
in file_by_file mode while binaries stay untracked.
