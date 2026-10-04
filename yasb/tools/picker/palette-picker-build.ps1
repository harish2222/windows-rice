# Rebuilds the Rust palette picker.
#
# This used to drive PyInstaller over palette-picker.py. That picker has been
# retired: the Rust one under tools/picker/palette-picker/ replaced it, and
# config.yaml has pointed at its target\release output for a while. The
# filename is unchanged so tools/setup/yasb-setup.ps1 still works.
#
# Nothing is copied anywhere. config.yaml launches the exe from cargo's own
# output directory, so building in place is the whole job — which is one more
# reason the PyInstaller step, whose only purpose was to produce a
# single-file exe elsewhere, no longer has a reason to exist.

$ErrorActionPreference = 'Stop'
$projectDir = Join-Path $PSScriptRoot 'palette-picker'

Push-Location $projectDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
} finally {
    Pop-Location
}

$built = Join-Path $projectDir 'target\release\palette-picker.exe'
if (-not (Test-Path -LiteralPath $built)) {
    throw "cargo reported success but $built is missing"
}
Write-Host "built   : $built"