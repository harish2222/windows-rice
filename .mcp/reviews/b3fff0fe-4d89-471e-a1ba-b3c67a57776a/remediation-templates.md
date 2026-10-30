# Remediation Templates — run b3fff0fe-4d89-471e-a1ba-b3c67a57776a

**Author -: HKDEVS / Project -: Window-Rice** · Detection-only: templates below are proposals awaiting operator approval. None have been applied.

## R-T1 · SHA-pin all workflow actions (fixes CI-ACTIONS-NOT-SHA-PINNED, HIGH — gate FAIL driver)

Scope: `.github/workflows/ci.yml`, `.github/workflows/release.yml`. Pattern applied to every `uses:`:

```yaml
# before
- uses: actions/checkout@v4
- uses: dtolnay/rust-toolchain@stable
# after (SHA resolved at apply time from the release the tag points to today)
- uses: actions/checkout@<full-commit-sha> # v4.3.0
- uses: dtolnay/rust-toolchain@<full-commit-sha> # stable (watch: dtolnay tags move; keep the branch-name comment)
```

Notes:
- `dtolnay/rust-toolchain@stable` is a *mutable ref to a third-party action*; after pinning, add Dependabot (`github-actions` ecosystem, pin true) or a periodic `pin-actions` refresh so SHAs don't rot.
- Effort: ~8 lines + config. Risk of apply: near-zero (CI re-runs prove it).

## R-T2 · Pin bootstrap endpoints before `iex` (fixes BS-REMOTE-EVAL-UNPINNED, HIGH)

`install.ps1` — replace direct `irm main | iex` with a tag-pinned route + fail-closed hash check:

```powershell
param(
  [string]$Version = 'v1.0.3',        # pin default to a release, not @main
  [switch]$Unpinned                   # opt-out for canary testing only
)
$base = if ($Unpinned) { 'main' } else { $Version }
$scriptUri  = "https://raw.githubusercontent.com/harish2222/windows-rice/$base/install.ps1"
$sumsUri    = "https://raw.githubusercontent.com/harish2222/windows-rice/$base/SHA256SUMS.txt" # or the release asset
$script = (Invoke-WebRequest -Uri $scriptUri -UseBasicParsing).Content
if (-not $Unpinned) {
  $expected = (Invoke-WebRequest -Uri $sumsUri -UseBasicParsing).Content |
    Where-Object { $_ -match 'install\.ps1' } |
    ForEach-Object { ($_ -split '\s+')[0] }
  $actual = [BitConverter]::ToString(
    [System.Security.Cryptography.SHA256]::Create().ComputeHash(
      [System.Text.Encoding]::UTF8.GetBytes($script))).Replace('-','').ToLowerInvariant()
  if ($actual -ne $expected) { throw "bootstrap script hash mismatch: $actual" }
}
Invoke-Expression $script
```

`installer/src/bootstrap.rs` — same doctrine for scoop/choco/uv:
- scoop: switch `irm https://get.scoop.sh | Invoke-Expression` to the pinned **anjali-scoop installer** alternative? No — keep get.scoop.sh but accept vendor pattern and note it, OR invoke scoop's `Invoke-Expression` after verifying `get.scoop.sh` charset — pragmatic template: pass `CI=1` and vendor SHA constants into the script builder.
- chocolatey: keep `community.chocolatey.org` eval only behind an explicit `--chocolatey` opt-in (self-evidencing).
- uv: `astral.sh/uv/install.ps1` → fetch `https://astral.sh/uv/latest` and pin, or use `pip install uv` from the already-pinned toolchain.

## R-T3 · Verify SHA256SUMS.txt before extraction (fixes ARTIFACT-ZIP-NO-HASH-CHECK, HIGH)

`install.ps1` archive block:

```powershell
$sums = Invoke-WebRequest -Uri "$releaseBase/SHA256SUMS.txt" -UseBasicParsing
$expected = ($sums.Content -split "`n") |
    Where-Object { $_ -match [regex]::Escape('rice-installer-windows-x64.zip') } |
    ForEach-Object { ($_ -split '\s+')[0] }
$actual = (Get-FileHash -Path $tempZip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($expected -and $actual -ne $expected.ToLowerInvariant()) {
    Remove-Item $tempZip -Force
    throw "installer zip hash mismatch: expected $expected got $actual"
}
Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force
```

## R-T4 · (optional) WinUtil wrapper confirmation (also MEDIUM EXT-TOOL-REMOTE-EVAL-WINUTIL)

```xsh
def winutil []:
    [input("run WinUtil (irm https://christitus.com/win | iex)? [y/N] ") == 'y'] and \
        subprocess.run(["powershell", "-NoProfile", "-Command", "irm https://christitus.com/win | iex"])
```
Alternative: accept as documented vendor pattern (risk acceptance, no change).

## R-T5 · (optional/decision) untrack bin/*.exe (LOW BIN-TRACKED-EXES)

```powershell
git rm --cached bin/rice-installer.exe bin/term-launch.exe
Add-Content .gitignore "`ebin/*.exe`nbin/term-launch.exe"
```
Side benefit: `security_run_pr_gate` un-OOMs for future recent_changes runs (this run's crash root).

## R-T6 · Risk-acceptance tickets (policy `exceptions.require_ticket = true`)

Write .mcp/tickets/ACCEPT-<ID>.md with ID, justification, reviewer (Owner — roles list is configurable), expiry.
Candidates: EXEC-POLICY-WIDEN, EXT-TOOL-REMOTE-EVAL-WINUTIL, BIN-TRACKED-EXES.

## Not remediated (verified controls, no action)

- FS-REVOKE-RESIDUAL — v1.0.3 linker guard + `test_unlink_symlink_to_source_restores_user_backup` hold.
- SUBPROC-ARGV-FIXED, CI-GHENV-ASSET-URL-WRITE, REL-DISPATCH-TAG-INPUT, CONTAINER-RUSTUP-EVAL, TERMLAUNCH-NAME-SPAWN — informational.
- Dependency set — OSV-clean as of 2026-10-30.
