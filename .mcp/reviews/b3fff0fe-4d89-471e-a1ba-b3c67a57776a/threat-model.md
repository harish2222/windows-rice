# Threat Model — windows-rice v1.0.3 (run b3fff0fe-4d89-471e-a1ba-b3c67a57776a)

**Date**: 2026-10-30 · **Status**: FILLED (detection-only review) · **Surfaces**: infra, web

## 1. Asset Inventory

| Asset | Sensitivity | Owner |
|---|---|---|
| User dotfiles & generated backups (revoke ledger) | HIGH | end user |
| User's shell/process integrity (any code run by installer) | CRITICAL | end user |
| GitHub release distribution channel (assets + SHA256SUMS) | HIGH | HKDEVS |
| CI secrets (GITHUB_TOKEN scope) | CRITICAL | HKDEVS |
| Symlink/junction targets (user + repo paths) | HIGH | end user / repo |

## 2. Trust Boundaries

- [x] B1: raw.githubusercontent.com / get.scoop.sh (`main`, mutable) → user's PowerShell — **irm|iex crosses here unvalidated**
- [x] B2: downloaded release zip → extracted `rice-installer.exe` → no hash check before extract/run
- [x] B3: GitHub events (workflow_dispatch, release asset URLs) → CI runner env (GITHUB_ENV write)
- [x] B4: installer process → user filesystem (remove_dir_all / remove_file / symlink unlink+restore)
- [x] B5: installer → subprocesses (powershell -Command, komorebic, glazewm, pwsh, git) — fixed-string args

## 3. Data Flow (L0/L1)

L0: user shell → bootstrap (irm|iex install.ps1) → installer TUI → dotfile render/symlink → komorebi/glazewm/yasb/otty.
L1: bootstrap opts `/Local` → local copy flow; install flow renders configs from repo, links into HOME, spawn worker runs per-component commands with CI=1; revoke flow walks ledger, unlinks links, classifies real-file vs symlink target (v1.0.3 guard), restores backups, removes dirs, prints `Full revoke complete:` summary.

## 4. STRIDE

| Component | S | T | R | I | D | E |
|---|---|---|---|---|---|---|
| irm\|iex bootstrap | spoofed origin (no pinning) | **tampered script E** (T1195) | none | none | CDN outage = install failure | **E via eval-of-remote** |
| Zip→exe artifact | no provenance pin | **swap without hash check** (T1553.002) | none | none | corrupt zip crash | E if hot path hijacked |
| Rust installer spawns | — | fixed literals OK | logs ok | low | runaway child w/o timeout | PATH-ordering only (T1574.007, mitigated: CI=1 fixed cmds) |
| Revoke engine (linker.rs) | — | **deletion of wrong targets (junction collapse) — fixed this cycle** | ledger provides audit | none | user data loss if guard regresses | none |
| CI release pipeline | tag-pinned actions (not SHA) | workflow_dispatch self-input (main-only, trusted) | Actions history | asset URL into env (property string) | mvn cache poisoning unlikely | untrusted-code-exec: absent (no pull_request_target + artifact pattern) |

## 5. PASTA

1. **Business objective**: trustworthy one-command dotfile install/uninstall.
2. **Scope**: bootstrap script, installer, linker, CI.
3. **Decomposition**: entry points = install.ps1 (all flows), installer CLI args, GitHub events.
4. **Threat actors**: repo compromise (typosquatted upstream via mutable `main`), MITM of artifact download, malicious PR (mitigated CI design), user-space symlink attacker.
5. **Vulnerabilities**: unpinned irm|iex; no artifact hash pre-check (installer or bootstrap); tag-not-SHA actions.
6. **Risk**: HIGH (bootstrap eval chain), MEDIUM (artifact integrity, revoke regressions), LOW (CI env write, policy change).

## 6. ATT&CK Mapping

- T1195.002 Compromise Software Supply Chain (mutable `main` bootstrap)
- T1059.001 PowerShell (iex eval of remote)
- T1553.002 Code Signing / hash bypass (no Get-FileHash validation)
- T1574.007 PATH interception (installer PATH augmentation — fixed commands limit impact)
- T1069 Permissions groups / T1547 not observed

## 7. Detection Gaps

This scan cannot catch: runtime behavior of komorebi/glazewm receiving configs, WASM/emoji payloads in themes, and social engineering of users running `irm|iex` themselves (documented, by-design convenience pattern of the ecosystem).
