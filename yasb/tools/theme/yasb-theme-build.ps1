# Rebuild + deploy yasb-theme.exe (run from anywhere).
#   pwsh -File C:\Users\haris\.config\yasb\tools\theme\yasb-theme-build.ps1
$ErrorActionPreference = 'Stop'
# The crate lives next to this script: <repo>\yasb\tools\theme\yasb-theme
$projectDir = Join-Path $PSScriptRoot 'yasb-theme'
Push-Location $projectDir
try {
    cargo build --release
}
finally {
    Pop-Location
}
$built = Join-Path $projectDir 'target\release\yasb-theme.exe'
$dest = Join-Path $PSScriptRoot 'yasb-theme.exe'
Copy-Item -LiteralPath $built -Destination $dest -Force
# silent-run is a separate tool with no source in this repo (shim-only, see
# scoop); refresh it only if this crate ever builds one again.
$builtSilent = Join-Path $projectDir 'target\release\silent-run.exe'
if (Test-Path -LiteralPath $builtSilent) {
    Copy-Item -LiteralPath $builtSilent -Destination 'C:\Users\haris\scoop\shims\silent-run.exe' -Force
}
$current = & $dest current
if ($LASTEXITCODE -ne 0) { throw 'smoke test failed: yasb-theme.exe current' }
Write-Output "deployed : $dest"
Write-Output "active   : $current"
