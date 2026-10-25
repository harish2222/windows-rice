# Rangalipi one-shot replication script — turns a fresh machine into this rice.
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\setup\yasb-setup.ps1
#
# Run from a clone of the Rangalipi repository.
# Builds the companion tools (pure Rust workspace, zero Python runtime/build dependencies),
# deploys the bar configuration, mandala artwork, GlazeWM + Komorebi window manager configs,
# adapts user paths dynamically, and verifies the entire chain.
#
# Idempotent: safe to re-run any time. Existing live files are backed up with a timestamp.

$repoRoot = Split-Path (Split-Path $PSScriptRoot)
$yasbDir = Join-Path $HOME '.config\yasb'
$glazeDir = Join-Path $HOME '.glzr\glazewm'
$komorebiDir = Join-Path $HOME '.config\komorebi'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
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

function Backup-File($path) {
    if (Test-Path -LiteralPath $path) {
        $bak = "$path.bak-$stamp"
        Copy-Item -LiteralPath $path -Destination $bak -Force
        Write-Output ("  backed up: {0} -> {1}" -f $path, $bak)
    }
}

function Deploy-File($src, $dst) {
    if (-not (Test-Path -LiteralPath $src)) { return }
    $srcFull = (Get-Item -LiteralPath $src).FullName
    $dstFull = [System.IO.Path]::GetFullPath($dst)
    if ($srcFull -eq $dstFull) { return }
    $d = Split-Path $dst
    if ($d -and -not (Test-Path -LiteralPath $d)) {
        New-Item $d -ItemType Directory -Force | Out-Null
    }
    Copy-Item -LiteralPath $src -Destination $dst -Force
}

function Deploy-Adapted-File($src, $dst, [hashtable]$replacements) {
    if (-not (Test-Path -LiteralPath $src)) { return }
    $srcFull = (Get-Item -LiteralPath $src).FullName
    $dstFull = [System.IO.Path]::GetFullPath($dst)
    $content = [System.IO.File]::ReadAllText($srcFull, [System.Text.Encoding]::UTF8)
    foreach ($k in $replacements.Keys) {
        $content = $content.Replace($k, $replacements[$k])
    }
    $d = Split-Path $dstFull
    if ($d -and -not (Test-Path -LiteralPath $d)) {
        New-Item $d -ItemType Directory -Force | Out-Null
    }
    [System.IO.File]::WriteAllText($dstFull, $content, [System.Text.Encoding]::UTF8)
}

Step 'paths' {
    foreach ($d in @(
        $yasbDir,
        $glazeDir,
        $komorebiDir,
        (Join-Path $yasbDir 'tools'),
        (Join-Path $yasbDir 'assets'),
        (Join-Path $yasbDir 'screenshot'),
        (Join-Path $HOME 'scoop\shims')
    )) {
        if (-not (Test-Path -LiteralPath $d)) {
            New-Item $d -ItemType Directory -Force | Out-Null
        }
    }
    foreach ($f in @('config.yaml', 'styles.css', 'Cargo.toml')) {
        if (-not (Test-Path -LiteralPath (Join-Path $repoRoot $f))) {
            throw ("repo file missing: {0}" -f $f)
        }
    }
    Write-Output ("  repo : {0}" -f $repoRoot)
    Write-Output ("  yasb : {0}" -f $yasbDir)
}

Step 'prerequisites (warn-only)' {
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
    if ($nf -eq 0) {
        Write-Warning 'no Nerd Fonts detected. Bar icons require JetBrainsMono NF or FiraCode NF: scoop bucket add nerd-fonts; scoop install nerd-fonts/JetBrainsMono-NF nerd-fonts/FiraCode-NF-Mono'
    }
}

Step 'back up live files' {
    Backup-File (Join-Path $yasbDir 'config.yaml')
    Backup-File (Join-Path $yasbDir 'styles.css')
    Backup-File (Join-Path $glazeDir 'config.yaml')
    Backup-File (Join-Path $HOME 'komorebi.json')
    Backup-File (Join-Path $komorebiDir 'komorebi.json')
    Backup-File (Join-Path $komorebiDir 'komorebi-startup.ps1')
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

Step 'build companion tools (pure rust workspace)' {
    Write-Output '  building release workspace (saka, saka-popup, palette-picker, keybind, theme, pomodoro, komorebi-sync, silent-run)...'
    Push-Location $repoRoot
    try {
        cargo build --workspace --release
        if ($LASTEXITCODE -ne 0) { throw 'cargo build --workspace --release failed' }
    } finally {
        Pop-Location
    }
    $requiredExes = @('yasb-theme.exe', 'silent-run.exe', 'palette-picker.exe', 'saka-popup.exe', 'saka.exe', 'keybind.exe', 'pomodoro.exe')
    foreach ($rx in $requiredExes) {
        $p = Join-Path $repoRoot ("target\release\" + $rx)
        if (-not (Test-Path -LiteralPath $p)) {
            throw ("expected release binary missing: {0}" -f $p)
        }
    }
    Write-Output '  all release binaries compiled successfully'
}

Step 'deploy silent-run shim' {
    $srcSilent = Join-Path $repoRoot 'target\release\silent-run.exe'
    $shimDir = Join-Path $HOME 'scoop\shims'
    if (Test-Path -LiteralPath $shimDir) {
        Copy-Item -LiteralPath $srcSilent -Destination (Join-Path $shimDir 'silent-run.exe') -Force
        Write-Output ("  deployed silent-run to: {0}" -f (Join-Path $shimDir 'silent-run.exe'))
    }
    # Also place next to theme tool and local bin
    $localBin = Join-Path $HOME '.local\bin'
    if (-not (Test-Path -LiteralPath $localBin)) { New-Item $localBin -ItemType Directory -Force | Out-Null }
    Copy-Item -LiteralPath $srcSilent -Destination (Join-Path $localBin 'silent-run.exe') -Force
    Refresh-Path
}

Step 'deploy tools to live yasb dir' {
    # Ensure tool target/release and root directories match invocation paths
    $binMap = @(
        @('target\release\yasb-theme.exe', 'tools\theme\yasb-theme.exe'),
        @('target\release\yasb-theme.exe', 'tools\theme\yasb-theme\target\release\yasb-theme.exe'),
        @('target\release\palette-picker.exe', 'tools\picker\palette-picker.exe'),
        @('target\release\palette-picker.exe', 'tools\picker\palette-picker\target\release\palette-picker.exe'),
        @('target\release\saka.exe', 'tools\saka\target\release\saka.exe'),
        @('target\release\saka-popup.exe', 'tools\saka-popup\target\release\saka-popup.exe'),
        @('target\release\keybind.exe', 'tools\keybind\keybind.exe'),
        @('target\release\keybind.exe', 'tools\keybind\target\release\keybind.exe'),
        @('target\release\pomodoro.exe', 'tools\pomodoro\target\release\pomodoro.exe'),
        @('target\release\komorebi-sync.exe', 'tools\komorebi-sync\target\release\komorebi-sync.exe'),
        @('target\release\screen-shot.exe', 'tools\screen-shot\target\release\screen-shot.exe')
    )
    foreach ($bm in $binMap) {
        $src = Join-Path $repoRoot $bm[0]
        $dst = Join-Path $yasbDir $bm[1]
        Deploy-File $src $dst
    }

    # Deploy helper scripts and catalogs
    $miscMap = @(
        @('tools\theme\yasb-theme-shell.ps1', 'tools\theme\yasb-theme-shell.ps1'),
        @('tools\picker\palette-themes.json', 'tools\picker\palette-themes.json'),
        @('tools\saka\saka-today.ps1', 'tools\saka\saka-today.ps1'),
        @('tools\theme\yasb-theme-current.bat', 'tools\theme\yasb-theme-current.bat'),
        @('tools\theme\yasb-theme-next.bat', 'tools\theme\yasb-theme-next.bat'),
        @('tools\theme\yasb-theme-prev.bat', 'tools\theme\yasb-theme-prev.bat')
    )
    foreach ($mm in $miscMap) {
        $src = Join-Path $repoRoot $mm[0]
        $dst = Join-Path $yasbDir $mm[1]
        if (Test-Path -LiteralPath $src) {
            Deploy-File $src $dst
        }
    }
    Write-Output '  companion tools and scripts deployed to live yasb dir'
}

Step 'deploy bar files with path adaptation' {
    $homeEscapedYaml = ($HOME.TrimEnd('\') -replace '\\', '\\\\')
    $homeFwdSlash = ($HOME.TrimEnd('\') -replace '\\', '/')

    # Adapt config.yaml paths for current user
    Deploy-Adapted-File `
        (Join-Path $repoRoot 'config.yaml') `
        (Join-Path $yasbDir 'config.yaml') `
        @{
            'C:\\Users\\haris' = $homeEscapedYaml;
            'C:/Users/haris' = $homeFwdSlash;
        }

    # Adapt styles.css motif URLs for current user
    Deploy-Adapted-File `
        (Join-Path $repoRoot 'styles.css') `
        (Join-Path $yasbDir 'styles.css') `
        @{
            'C:/Users/haris/.config/yasb/assets' = "$homeFwdSlash/.config/yasb/assets";
            'C:\\Users\\haris\\.config\\yasb\\assets' = "$homeEscapedYaml\\.config\\yasb\\assets";
            'C:/Users/haris' = $homeFwdSlash;
            'C:\\Users\\haris' = $homeEscapedYaml;
        }

    # Deploy assets (mandala, motifs, raw-art)
    $assetsSrc = Join-Path $repoRoot 'assets'
    $assetsDst = Join-Path $yasbDir 'assets'
    if ((Test-Path $assetsSrc) -and ((Get-Item $assetsSrc).FullName -ne (Get-Item $assetsDst -ErrorAction SilentlyContinue).FullName)) {
        robocopy $assetsSrc $assetsDst /E /NFL /NDL /NJH /NJS | Out-Null
        Write-Output '  assets (mandala, motifs, raw-art) deployed'
    }

    # Deploy screenshots
    $screenshotSrc = Join-Path $repoRoot 'screenshot'
    $screenshotDst = Join-Path $yasbDir 'screenshot'
    if ((Test-Path $screenshotSrc) -and ((Get-Item $screenshotSrc).FullName -ne (Get-Item $screenshotDst -ErrorAction SilentlyContinue).FullName)) {
        robocopy $screenshotSrc $screenshotDst /E /NFL /NDL /NJH /NJS | Out-Null
        Write-Output '  screenshots deployed'
    }

    # Deploy auxiliary configs & design docs
    foreach ($aux in @('DESIGN.md', 'rangalipi-options.css', 'yasb_colors.css')) {
        $sp = Join-Path $repoRoot $aux
        if (Test-Path $sp) {
            Deploy-File $sp (Join-Path $yasbDir $aux)
        }
    }

    Write-Output '  bar config, stylesheet, and art deployed'
}

Step 'deploy window-manager configs' {
    $homeEscapedJson = ($HOME.TrimEnd('\') -replace '\\', '\\\\')

    # Deploy GlazeWM config
    Deploy-Adapted-File `
        (Join-Path $repoRoot 'configs\glazewm\config.yaml') `
        (Join-Path $glazeDir 'config.yaml') `
        @{
            'C:\\Users\\haris' = $homeEscapedJson;
        }

    # Deploy Komorebi configs (with proper application config path)
    Deploy-Adapted-File `
        (Join-Path $repoRoot 'configs\komorebi\komorebi.json') `
        (Join-Path $komorebiDir 'komorebi.json') `
        @{
            'C:\\Users\\haris' = $homeEscapedJson;
        }
    Deploy-File (Join-Path $komorebiDir 'komorebi.json') (Join-Path $HOME 'komorebi.json')

    foreach ($kf in @('komorebi-startup.ps1', 'komorebi.bar.json', 'applications.json')) {
        $sp = Join-Path $repoRoot ("configs\komorebi\" + $kf)
        if (Test-Path $sp) {
            Deploy-File $sp (Join-Path $komorebiDir $kf)
        }
    }
    Write-Output '  glazewm + komorebi configs deployed with adapted paths'
}

Step 'verify chain' {
    # Theme CLI
    $exeTheme = Join-Path $yasbDir 'tools\theme\yasb-theme.exe'
    $cur = & $exeTheme current
    if ($LASTEXITCODE -ne 0) { throw 'yasb-theme current failed' }
    $n = ((& $exeTheme list) | Measure-Object -Line).Lines
    Write-Output ("  theme tool ok: {0} themes available, active: {1}" -f $n, $cur)
    if ($n -lt 21) { throw ("expected at least 21 themes, got {0}" -f $n) }

    # Saka CLI
    $saka = Join-Path $yasbDir 'tools\saka\target\release\saka.exe'
    $sakaOut = & $saka (Get-Date -Format 'yyyy-MM-dd') --long
    if ($LASTEXITCODE -ne 0) { throw 'saka.exe smoke failed' }
    Write-Output ("  saka ok: {0}" -f $sakaOut)

    # Saka popup smoke test
    $sakaPopup = Join-Path $yasbDir 'tools\saka-popup\target\release\saka-popup.exe'
    $null = & $sakaPopup --smoke
    if ($LASTEXITCODE -ne 0) { throw 'saka-popup.exe --smoke failed' }
    Write-Output '  saka-popup ok (headless smoke render passed)'

    # Palette picker smoke test
    $picker = Join-Path $yasbDir 'tools\picker\palette-picker\target\release\palette-picker.exe'
    $null = & $picker --smoke
    if ($LASTEXITCODE -ne 0) { throw 'palette-picker.exe --smoke failed' }
    Write-Output '  palette-picker ok (headless catalog & keyboard test passed)'

    # Pomodoro CLI
    $pomo = Join-Path $yasbDir 'tools\pomodoro\target\release\pomodoro.exe'
    $null = & $pomo tick
    if ($LASTEXITCODE -ne 0) { throw 'pomodoro.exe smoke failed' }
    Write-Output '  pomodoro ok'

    # Keybind existence
    $keybind = Join-Path $yasbDir 'tools\keybind\target\release\keybind.exe'
    if (-not (Test-Path $keybind)) { throw 'keybind.exe missing' }
    Write-Output '  keybind ok'

    # Check silent-run
    if (Has-Cmd 'silent-run') {
        Write-Output '  silent-run is on PATH'
    } else {
        Write-Warning 'silent-run is not yet on PATH in current session (restart terminal or ensure ~/scoop/shims or ~/.local/bin is in PATH)'
    }
}

Step 'shell helper (yt)' {
    if (-not (Test-Path $PROFILE)) { New-Item $PROFILE -ItemType File -Force | Out-Null }
    $line = '. "{0}"' -f (Join-Path $yasbDir 'tools\theme\yasb-theme-shell.ps1')
    if (-not (Select-String -Path $PROFILE -Pattern 'yasb-theme-shell' -Quiet)) {
        Add-Content $PROFILE $line
        Write-Output '  yt helper registered in $PROFILE'
    } else {
        Write-Output '  yt helper already in $PROFILE'
    }
}

Write-Output ""
Write-Output "================ SUMMARY ================"
$results | Format-Table -AutoSize | Out-String | Write-Output
if ($results.Result -match '^FAIL') { exit 1 }
Write-Output 'Replication complete!'
Write-Output 'To apply changes:'
Write-Output '  1. Restart YASB (or run: yasb)'
Write-Output '  2. Restart GlazeWM & Komorebi (alt+shift+r)'
Write-Output '  3. In a new terminal, run: yt list'
