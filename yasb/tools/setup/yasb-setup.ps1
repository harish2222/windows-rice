# One-shot setup / replication script for the YASB companion tooling.
#   powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\haris\.config\yasb\yasb-setup.ps1
# Installs missing toolchains (Rust, Python Qt6 stack), rebuilds every
# companion binary from source, and verifies the whole chain. Idempotent:
# safe to re-run any time. Never touches config.yaml / styles.css.
$yasbDir = Split-Path (Split-Path $PSScriptRoot)
$results = @()
function Step($name, [scriptblock]$body) {
    Write-Output ""
    Write-Output ("=== {0} ===" -f $name)
    try {
        & $body
        $script:results += [pscustomobject]@{ Step = $name; Result = 'PASS' }
        Write-Output ("[ok] {0}" -f $name)
    } catch {
        $script:results += [pscustomobject]@{ Step = $name; Result = 'FAIL: ' + $_.Exception.Message }
        Write-Output ("[FAIL] {0}: {1}" -f $name, $_.Exception.Message)
    }
}
function Refresh-Path {
    $m = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $u = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = "$m;$u"
}
function Has-Cmd($name) {
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

Step 'external apps (warn-only)' {
    $need = @(
        @('yasb', 'scoop install yasb'),
        @('komorebic', 'scoop install komorebi'),
        @('glazewm', 'scoop install glazewm'),
        @('cava', 'cava installer from github.com/karlstav/cava/releases')
    )
    foreach ($n in $need) {
        if (Has-Cmd $n[0]) { Write-Output ("  {0}: present" -f $n[0]) }
        else { Write-Output ("  {0}: MISSING -> {1}" -f $n[0], $n[1]) }
    }
    $faces = @()
    foreach ($h in @('HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts',
            'HKCU:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts')) {
        if (Test-Path $h) {
            $faces += @(Get-ItemProperty $h -ErrorAction SilentlyContinue |
                Get-Member -MemberType NoteProperty | Where-Object { $_.Name -match 'nerd' })
        }
    }
    $nf = $faces.Count
    Write-Output ("  nerd faces installed: {0}" -f $nf)
    if ($nf -eq 0) { throw 'no Nerd Fonts installed (picker/bar icons need at least one)' }
}

Step 'rust toolchain' {
    if (-not (Has-Cmd 'cargo')) {
        Write-Output '  installing Rust via winget...'
        winget install --id Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements
        Refresh-Path
    }
    if (-not (Has-Cmd 'cargo')) { throw 'cargo still missing after install' }
    Write-Output ("  cargo: {0}" -f (Get-Command cargo).Source)
}

Step 'python Qt6 stack' {
    if (-not (Has-Cmd 'python3')) { throw 'python3 not found (install via winget: Python.Python.3.13)' }
    python3 -c "import PyQt6.QtCore as q; print('  PyQt6 Qt', q.qVersion())" 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Output '  installing PyQt6 + PyInstaller...'
        python3 -m pip install --quiet PyQt6 PyInstaller
    }
    python3 -c "import PyQt6, PyInstaller; print('  modules ok')" 2>$null
    if ($LASTEXITCODE -ne 0) { throw 'PyQt6/PyInstaller import failed' }
}

Step 'build theme tool (rust)' {
    powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $yasbDir 'tools\theme\yasb-theme-build.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'yasb-theme-build.ps1 failed' }
}

Step 'build palette picker (qt6)' {
    powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $yasbDir 'tools\picker\palette-picker-build.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'palette-picker-build.ps1 failed' }
}

Step 'build font tool' {
    powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $yasbDir 'tools\picker\yasb-font-build.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'yasb-font-build.ps1 failed' }
}

Step 'snapshot shared configs' {
    Copy-Item "$HOME\.glzr\glazewm\config.yaml" (Join-Path $yasbDir 'configs\glazewm\config.yaml') -Force
    Copy-Item "$HOME\komorebi.json" (Join-Path $yasbDir 'configs\komorebi\komorebi.json') -Force
    Copy-Item "$HOME\.config\komorebi\komorebi-startup.ps1" (Join-Path $yasbDir 'configs\komorebi\komorebi-startup.ps1') -Force
    Write-Output '  live glazewm + komorebi configs snapshotted into configs\'
}

Step 'verify chain' {
    $exe = Join-Path $yasbDir 'tools\theme\yasb-theme.exe'
    $cur = & $exe current
    if ($LASTEXITCODE -ne 0) { throw 'yasb-theme current failed' }
    $n = ((& $exe list) | Measure-Object -Line).Lines
    Write-Output ("  theme tool ok: {0} themes, active: {1}" -f $n, $cur)
    if (-not (Has-Cmd 'silent-run')) { throw 'silent-run not on PATH' }
    Write-Output '  silent-run on PATH'
    $pj = Join-Path $yasbDir 'tools\picker\palette-themes.json'
    if (-not (Test-Path $pj)) { throw 'palette-themes.json missing' }
    $p = Start-Process (Join-Path $yasbDir 'tools\picker\palette-picker.exe') -PassThru
    Start-Sleep -Seconds 8
    $alive = $null -ne (Get-Process -Id $p.Id -ErrorAction SilentlyContinue)
    Get-Process -Name palette-picker -ErrorAction SilentlyContinue | Stop-Process -Force
    if (-not $alive) { throw 'picker exe died on boot (bundled Qt broken)' }
    Write-Output '  picker exe boots bundled Qt6'
}

Step 'shell helper (yt)' {
    if (-not (Test-Path $PROFILE)) { New-Item $PROFILE -ItemType File -Force | Out-Null }
    $line = '. "{0}"' -f (Join-Path $yasbDir 'tools\theme\yasb-theme-shell.ps1')
    if (-not (Select-String -Path $PROFILE -Pattern 'yasb-theme-shell' -Quiet)) {
        Add-Content $PROFILE $line
        Write-Output '  yt helper added to profile'
    } else { Write-Output '  yt helper already in profile' }
}

Write-Output ""
Write-Output "================ SUMMARY ================"
$results | Format-Table -AutoSize | Out-String | Write-Output
if ($results.Result -match '^FAIL') { exit 1 }
Write-Output 'setup complete: restart the shell, then run: yt list'
