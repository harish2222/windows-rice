# PR Gate Result — run b3fff0fe-4d89-471e-a1ba-b3c67a57776a

**Verdict: FAIL** (detection-only; no files modified) · **Date:** 2026-10-30 · **Base:** 45fd6f9 (v1.0.1) → **Head:** HEAD (f9e878e, v1.0.3)

## Provenance — why this was executed locally

`security_run_pr_gate` (mode recent_changes) crashed the security-mcp server twice:

1. `File not found: .mcp[path]` — pre-policy-fix (policy file absent) — 2026-10-30
2. `Connection closed: MCP server process exited with code 134: heap out of memory` — with policy present. Root cause identified: the diff range contains two **tracked binary files** (`bin/rice-installer.exe`, `bin/term-launch.exe`); materializing the binary blob diff blew the Node heap.

Server de-registered after the crash (`Unknown tool` on all security-mcp calls). Per operator instruction, the gate was completed **locally** using the same policy thresholds and detector classes, with the scan scoped `file_by_file` over 31 code/config/test/workflow targets (binaries excluded — unscannable content, and each binary is re-built and SHA256SUMS.txt-hashed by CI at release). All evidence below is reproducible via plain `rg`/git/OSV queries.

## Scan results (policy: .mcp/policies/security-policy.json)

| Check | Threshold | Result | Blocking? |
|---|---|---|---|
| sast | CRITICAL | 0 critical, 2 HIGH, 2 MEDIUM | no |
| iac | HIGH+CRITICAL | **1 HIGH** | **YES — FAIL driver** |
| secrets | HIGH+CRITICAL | clean (rg token sweep, repo-wide excl. bin/.git/.mcp) | no |
| dependency | CRITICAL | clean — OSV: ratatui 0.29.0 = 0 advisories; crossterm 0.28.1 = 0 advisories; control case time 0.1.42 → RUSTSEC-2020-0071 proves empty=clean; 78 locked pkgs; runtime deps ratatui/crossterm/serde/serde_json/anyhow | no |
| artifacts_required | threat-model on install.ps1+installer/** changes | satisfied (.mcp/reviews/<run>/threat-model.md) | no |

## Verdict drivers

**FAIL** — IAC scan tripped at HIGH threshold by `CI-ACTIONS-NOT-SHA-PINNED` (tag-pinned actions incl. third-party `dtolnay/rust-toolchain@stable`).

## Findings summary

| ID | Sev | One-line |
|---|---|---|
| CI-ACTIONS-NOT-SHA-PINNED | HIGH (blocks) | All workflow actions pinned by tag, not SHA |
| BS-REMOTE-EVAL-UNPINNED | HIGH | irm\|iex of mutable endpoints (install.ps1 ×6 sites + bootstrap.rs scoop/choco/uv) |
| ARTIFACT-ZIP-NO-HASH-CHECK | HIGH | Downloaded zip expanded+run with no Get-FileHash (SHA256SUMS exists at release — 3-line fix) |
| EXT-TOOL-REMOTE-EVAL-WINUTIL | MEDIUM | xonsh/nushell WinUtil irm\|iex wrappers |
| EXEC-POLICY-WIDEN | MEDIUM | Set-ExecutionPolicy RemoteSigned CurrentUser persists without revert |
| CI-GHENV-ASSET-URL-WRITE | LOW | GITHUB_ENV property write, no injection sink |
| REL-DISPATCH-TAG-INPUT | LOW | main-only workflow_dispatch self-input |
| FS-REVOKE-RESIDUAL | LOW | v1.0.3 guard + regression test verified (mitigated) |
| SUBPROC-ARGV-FIXED | LOW | Fixed-string subprocess args, no taint |
| CONTAINER-RUSTUP-EVAL | LOW | Container-isolated rustup bootstrap |
| TERMLAUNCH-NAME-SPAWN | LOW | Self-user launcher spawn semantics |
| BIN-TRACKED-EXES | LOW | Tracked exes can drift from source; caused gate OOM |

Full dataset with file:line evidence: `findings.json` (same directory).

## Required actions (detection_only — awaiting user approval, nothing applied)

1. **Apply (recommended, small diffs):** SHA-pin all `uses:` across ci.yml/release.yml; pin bootstrap endpoints to the release tag route; add SHA256SUMS.txt verification before `Expand-Archive`.
2. **Decide:** WinUtil wrappers (vendor pattern — keep/confirm/pin), untracking `bin/*.exe`.
3. **Upstream-able tooling bug:** security-mcp gate OOMs on binary-contained diffs — re-run only with file_by_file code targets (done here), and report upstream to AbrahamOO/security-mcp.

## Risk acceptance candidates (LOW/MEDIUM, no code change)

EXEC-POLICY-WIDEN, EXT-TOOL-REMOTE-EVAL-WINUTIL, BIN-TRACKED-EXES — acceptable with a one-line justification ticket each per policy exceptions (approval role: Owner).
